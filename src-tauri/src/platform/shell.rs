//! 打开文件：用系统默认程序，没有关联程序时退回记事本。

#[cfg(windows)]
pub fn open_path(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{PCWSTR, w};

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let file = PCWSTR(wide.as_ptr());
    // 作为命令行参数传给记事本时要加引号，路径里可能有空格
    let quoted: Vec<u16> = "\"".encode_utf16().chain(path.as_os_str().encode_wide()).chain("\"\0".encode_utf16()).collect();
    unsafe {
        // 返回值 <= 32 表示失败（比如 .toml 没有关联程序）
        let r = ShellExecuteW(None, w!("open"), file, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
        if r.0 as usize <= 32 {
            ShellExecuteW(None, w!("open"), w!("notepad.exe"), PCWSTR(quoted.as_ptr()), PCWSTR::null(), SW_SHOWNORMAL);
        }
    }
}

#[cfg(not(windows))]
pub fn open_path(_path: &std::path::Path) {}
