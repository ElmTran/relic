//! 记录和恢复前台窗口、取光标位置和进程名。HWND 存成 isize，方便跨线程保存。

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HWND, POINT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, IsIconic, IsWindow,
    SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::PWSTR;

/// 屏幕坐标（物理像素）下的文本光标：左边、上沿、下沿
#[derive(Debug, Clone, Copy)]
pub struct Caret {
    pub x: i32,
    pub top: i32,
    pub bottom: i32,
}

/// 窗口所在线程的系统光标。Chromium、Electron、微信等自绘控件通常拿不到
pub fn caret(hwnd: isize) -> Option<Caret> {
    unsafe {
        let tid = GetWindowThreadProcessId(HWND(hwnd as _), None);
        let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        GetGUIThreadInfo(tid, &mut info).ok()?;
        let r = info.rcCaret;
        if info.hwndCaret.is_invalid() || r.bottom <= r.top {
            return None;
        }
        let mut top_left = POINT { x: r.left, y: r.top };
        if !ClientToScreen(info.hwndCaret, &mut top_left).as_bool() {
            return None;
        }
        Some(Caret { x: top_left.x, top: top_left.y, bottom: top_left.y + (r.bottom - r.top) })
    }
}

/// 是不是终端窗口。终端里 Ctrl+C 是中断、Ctrl+A 不是全选，输入行也读不到，要走单独的流程
pub fn is_terminal(hwnd: isize) -> bool {
    const CLASSES: &[&str] = &["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS"];
    const PROCESSES: &[&str] = &[
        "WindowsTerminal.exe",
        "OpenConsole.exe",
        "conhost.exe",
        "wezterm-gui.exe",
        "alacritty.exe",
        "mintty.exe",
        "Tabby.exe",
        "Hyper.exe",
    ];
    CLASSES.contains(&class_name(hwnd).as_str())
        || process_name(hwnd).is_some_and(|p| PROCESSES.iter().any(|t| t.eq_ignore_ascii_case(&p)))
}

pub fn class_name(hwnd: isize) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(HWND(hwnd as _), &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// 是不是 Relic 自己的窗口（翻译窗口、设置窗口、浮标）
pub fn is_own(hwnd: isize) -> bool {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as _), Some(&mut pid)) };
    pid == std::process::id()
}

/// 窗口所属进程的文件名，如 "Code.exe"
pub fn process_name(hwnd: isize) -> Option<String> {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(HWND(hwnd as _), Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

pub fn foreground() -> Option<isize> {
    let h = unsafe { GetForegroundWindow() };
    (!h.is_invalid()).then_some(h.0 as isize)
}

/// 切回之前记录的窗口，等焦点真正生效
pub fn restore(hwnd: isize) -> Result<(), String> {
    let h = HWND(hwnd as _);
    unsafe {
        if !IsWindow(Some(h)).as_bool() {
            return Err("原窗口已经关闭".into());
        }
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let _ = SetForegroundWindow(h);
    }
    let start = Instant::now();
    while foreground() != Some(hwnd) {
        if start.elapsed() > Duration::from_millis(500) {
            return Err("无法切回原窗口".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
