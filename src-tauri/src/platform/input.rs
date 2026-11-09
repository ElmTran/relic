//! 模拟按键（SendInput）。注入的按键带 RELIC_MARK，M3 的键盘钩子据此忽略自己发的键。

use std::time::{Duration, Instant};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC,
    MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_A, VK_C, VK_CONTROL, VK_INSERT, VK_LWIN, VK_MENU, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_V,
};

pub const RELIC_MARK: usize = 0x5245_4C49; // "RELI"

/// AutoHotkey 的"菜单屏蔽键"：没有任何用途的虚拟键
const VK_MASK: VIRTUAL_KEY = VIRTUAL_KEY(0xE8);

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let scan = unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) } as u16;
    let mut flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    // 方向键、Insert 是扩展键，不带这个标志有的应用会按扫描码当成小键盘
    if vk == VK_RIGHT || vk == VK_INSERT {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: RELIC_MARK,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
}

/// 按下组合键：依次按下，再倒序松开
fn chord(keys: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| key(k, false)).collect();
    inputs.extend(keys.iter().rev().map(|&k| key(k, true)));
    send(&inputs);
}

pub fn ctrl_c() {
    chord(&[VK_CONTROL, VK_C]);
}

pub fn ctrl_v() {
    chord(&[VK_CONTROL, VK_V]);
}

pub fn ctrl_a() {
    chord(&[VK_CONTROL, VK_A]);
}

/// 终端的通用粘贴键：Windows Terminal 默认绑定，控制台、PSReadLine、mintty 也都支持。
/// （Ctrl+V 在 bash 里是"原样插入下一个字符"，只有终端接管时才是粘贴）
pub fn shift_insert() {
    chord(&[VK_SHIFT, VK_INSERT]);
}

/// → 键：把选区收起到末尾
pub fn right() {
    chord(&[VK_RIGHT]);
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000) != 0
}

/// Alt 热键触发后，原应用只看到 Alt 按下又松开，会激活菜单栏。
/// Alt 还按着时插一个无用按键，Alt 松开就不算单击。
pub fn mask_alt_if_down() {
    if is_down(VK_MENU) {
        send(&[key(VK_MASK, false), key(VK_MASK, true)]);
    }
}

/// 等热键的修饰键都松开，否则发出去的 Ctrl+C 会变成 Ctrl+Alt+C。返回是否等到了
pub fn wait_modifiers_released(timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if ![VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN].into_iter().any(is_down) {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
