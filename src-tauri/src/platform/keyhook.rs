//! 低级键盘钩子（WH_KEYBOARD_LL）：识别"快速按两下 Alt"和 Esc。只观察，不拦截任何按键。
//! 回调里只更新状态、把事件发给工作线程，必须很快返回，否则系统会摘掉钩子。

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, mpsc::Sender};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_SYSKEYDOWN,
};

#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    DoubleAlt,
    Escape,
}

/// 按下到松开超过这个时间就不算"单击"
const TAP_MAX: Duration = Duration::from_millis(300);

const VK_ESCAPE: u32 = 0x1B;
const VK_MENU: u32 = 0x12;
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;

static SENDER: OnceLock<Sender<HookEvent>> = OnceLock::new();
static INTERVAL_MS: AtomicU64 = AtomicU64::new(300);

#[derive(Default)]
struct State {
    /// Alt 按下的时刻；松开或被其他键打断前一直有值
    alt_down_at: Option<Instant>,
    /// Alt 按着期间按过其他键（Alt+Tab 之类）
    interrupted: bool,
    /// 上一次有效单击 Alt 松开的时刻
    last_tap: Option<Instant>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// 两次单击的最大间隔（配置热更新时调用）
pub fn set_interval(ms: u64) {
    INTERVAL_MS.store(ms, Ordering::Relaxed);
}

/// 在独立线程里安装钩子并跑消息循环。只会安装一次
pub fn start(tx: Sender<HookEvent>) {
    if SENDER.set(tx).is_err() {
        return;
    }
    std::thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        if let Err(e) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0) {
            eprintln!("[relic] 安装键盘钩子失败: {e}");
            return;
        }
        // 低级钩子要求安装线程有消息循环
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}
    });
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // 忽略所有注入的按键，包括 Relic 自己发的
        if kb.flags.0 & LLKHF_INJECTED.0 == 0 {
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            if let Some(event) = STATE.with_borrow_mut(|s| s.on_key(kb.vkCode, down, Instant::now())) {
                if let Some(tx) = SENDER.get() {
                    let _ = tx.send(event);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

impl State {
    fn on_key(&mut self, vk: u32, down: bool, now: Instant) -> Option<HookEvent> {
        let interval = Duration::from_millis(INTERVAL_MS.load(Ordering::Relaxed));
        if matches!(vk, VK_MENU | VK_LMENU | VK_RMENU) {
            if down {
                // 按住不放会不断重复 keydown，只认第一次
                if self.alt_down_at.is_none() {
                    self.alt_down_at = Some(now);
                    self.interrupted = false;
                    if self.last_tap.is_some_and(|t| now - t > interval) {
                        self.last_tap = None;
                    }
                }
                return None;
            }
            let pressed_at = self.alt_down_at.take()?;
            if self.interrupted || now - pressed_at > TAP_MAX {
                self.last_tap = None;
                return None;
            }
            if self.last_tap.take().is_some() {
                return Some(HookEvent::DoubleAlt);
            }
            self.last_tap = Some(now);
            return None;
        }
        if down {
            // 其他键打断：Alt 组合键不算单击，两次单击之间按了别的键也重新计数
            self.interrupted = true;
            self.last_tap = None;
            if vk == VK_ESCAPE {
                return Some(HookEvent::Escape);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(keys: &[(u32, bool, u64)]) -> Vec<String> {
        let mut s = State::default();
        let t0 = Instant::now();
        keys.iter()
            .filter_map(|&(vk, down, ms)| s.on_key(vk, down, t0 + Duration::from_millis(ms)))
            .map(|e| format!("{e:?}"))
            .collect()
    }

    const A: u32 = VK_LMENU;
    const TAB: u32 = 0x09;

    #[test]
    fn double_tap() {
        assert_eq!(run(&[(A, true, 0), (A, false, 80), (A, true, 200), (A, false, 260)]), ["DoubleAlt"]);
        // 右 Alt 也算，左右混用也算
        assert_eq!(run(&[(VK_RMENU, true, 0), (VK_RMENU, false, 80), (A, true, 200), (A, false, 260)]), ["DoubleAlt"]);
    }

    #[test]
    fn not_double_tap() {
        // 间隔太长
        assert!(run(&[(A, true, 0), (A, false, 80), (A, true, 500), (A, false, 560)]).is_empty());
        // 按住太久
        assert!(run(&[(A, true, 0), (A, false, 400), (A, true, 450), (A, false, 500)]).is_empty());
        // Alt+Tab 后再单击 Alt
        assert!(run(&[(A, true, 0), (TAB, true, 50), (A, false, 100), (A, true, 200), (A, false, 250)]).is_empty());
        // 两次单击之间按了别的键
        assert!(run(&[(A, true, 0), (A, false, 50), (0x41, true, 100), (A, true, 200), (A, false, 250)]).is_empty());
        // 按住 Alt 自动重复不算多次
        assert!(run(&[(A, true, 0), (A, true, 30), (A, true, 60), (A, false, 90)]).is_empty());
    }

    #[test]
    fn triple_tap_triggers_once() {
        let keys = [(A, true, 0), (A, false, 50), (A, true, 150), (A, false, 200), (A, true, 300), (A, false, 350)];
        assert_eq!(run(&keys), ["DoubleAlt"]);
    }

    #[test]
    fn escape() {
        assert_eq!(run(&[(VK_ESCAPE, true, 0), (VK_ESCAPE, false, 50)]), ["Escape"]);
    }
}
