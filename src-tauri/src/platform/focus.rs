//! 记录和恢复前台窗口。HWND 存成 isize，方便跨线程保存。

use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, IsWindow, SW_RESTORE, SetForegroundWindow, ShowWindow,
};

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
