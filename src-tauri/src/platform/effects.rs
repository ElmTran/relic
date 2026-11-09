//! 窗口效果。Acrylic 由 Tauri 的 windowEffects 配置（Win11 22H2+ 走 DWM 系统背景），
//! 这里只负责原生圆角。

#[cfg(windows)]
pub fn round_corners(win: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
    };

    let Ok(raw) = win.hwnd() else { return };
    let hwnd = HWND(raw.0 as _);
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

#[cfg(not(windows))]
pub fn round_corners(_win: &tauri::WebviewWindow) {}
