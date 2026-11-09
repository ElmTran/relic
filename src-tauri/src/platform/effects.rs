//! 窗口效果。Acrylic 由 Tauri 的 windowEffects 配置（Win11 22H2+ 走 DWM 系统背景），
//! 这里负责原生圆角，以及浮标"显示但不抢焦点"。

#[cfg(windows)]
fn hwnd(win: &tauri::WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    win.hwnd().ok().map(|h| windows::Win32::Foundation::HWND(h.0 as _))
}

#[cfg(windows)]
pub fn round_corners(win: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};

    let Some(hwnd) = hwnd(win) else { return };
    let pref = DWMWCP_ROUND;
    // 失败（如 Win10）时保持直角即可
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const _,
            std::mem::size_of_val(&pref) as u32,
        )
    };
}

/// 加上 WS_EX_NOACTIVATE：点击也不会激活，焦点留在原应用
#[cfg(windows)]
pub fn make_no_activate(win: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE};

    let Some(hwnd) = hwnd(win) else { return };
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize);
    }
}

/// 移到指定位置（物理像素）并置顶显示，不激活窗口
#[cfg(windows)]
pub fn show_no_activate(win: &tauri::WebviewWindow, x: i32, y: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos,
    };

    let Some(hwnd) = hwnd(win) else { return };
    let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) };
}

/// 隐藏用 show_no_activate 显示的窗口。必须也走 Win32：Tauri 不知道窗口被显示过，
/// 它的 hide() 会认为状态没变而什么都不做
#[cfg(windows)]
pub fn hide_window(win: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    let Some(hwnd) = hwnd(win) else { return };
    let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
}

#[cfg(not(windows))]
pub fn hide_window(win: &tauri::WebviewWindow) {
    let _ = win.hide();
}

#[cfg(not(windows))]
pub fn round_corners(_win: &tauri::WebviewWindow) {}

#[cfg(not(windows))]
pub fn make_no_activate(_win: &tauri::WebviewWindow) {}

#[cfg(not(windows))]
pub fn show_no_activate(win: &tauri::WebviewWindow, x: i32, y: i32) {
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = win.show();
}
