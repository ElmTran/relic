//! 剪贴板：快照与恢复（尽量保留所有基于 HGLOBAL 的格式）、写文本、读文本、序列号。
//! 临时写入的内容带上排除格式，不进 Win+V 历史和云剪贴板。
//! owner 是剪贴板所有者窗口的 HWND（用翻译窗口），为空时 SetClipboardData 可能失败。

mod imp {
    use std::time::Duration;

    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
        OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
    use windows::core::w;

    const CF_UNICODETEXT: u32 = 13;

    /// 不是 HGLOBAL 的格式（GDI 句柄、私有句柄等），快照时跳过
    fn is_hglobal_format(f: u32) -> bool {
        !matches!(f, 2 | 3 | 9 | 14 | 0x80 | 0x82 | 0x83 | 0x8E | 0x200..=0x3FF)
    }

    /// 剪贴板快照
    pub struct Snapshot(Vec<(u32, Vec<u8>)>);

    /// 打开剪贴板期间持有，离开作用域时关闭
    struct Opened;

    impl Opened {
        /// 剪贴板可能正被别的程序占用，重试一会
        fn open(owner: isize) -> Result<Self, String> {
            let owner = Some(HWND(owner as _));
            for _ in 0..30 {
                if unsafe { OpenClipboard(owner) }.is_ok() {
                    return Ok(Opened);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err("剪贴板被其他程序占用".into())
        }
    }

    impl Drop for Opened {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    pub fn sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn snapshot(owner: isize) -> Result<Snapshot, String> {
        let _open = Opened::open(owner)?;
        let mut items = Vec::new();
        let mut format = 0;
        loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if !is_hglobal_format(format) {
                continue;
            }
            let Ok(handle) = (unsafe { GetClipboardData(format) }) else { continue };
            if let Some(bytes) = unsafe { read_hglobal(HGLOBAL(handle.0)) } {
                items.push((format, bytes));
            }
        }
        Ok(Snapshot(items))
    }

    pub fn restore(owner: isize, snap: &Snapshot) -> Result<(), String> {
        let _open = Opened::open(owner)?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        for (format, bytes) in &snap.0 {
            unsafe { set_bytes(*format, bytes) };
        }
        // 恢复的是原来就有的内容，不要在历史里再出现一次
        unsafe { mark_excluded() };
        Ok(())
    }

    /// 写入文本。transient=true 时不进 Win+V 历史和云剪贴板（回填、取词用的临时内容）
    pub fn set_text(owner: isize, text: &str, transient: bool) -> Result<(), String> {
        let _open = Opened::open(owner)?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2) };
        if !unsafe { set_bytes(CF_UNICODETEXT, bytes) } {
            return Err("写入剪贴板失败".into());
        }
        if transient {
            unsafe { mark_excluded() };
        }
        Ok(())
    }

    pub fn get_text(owner: isize) -> Option<String> {
        let _open = Opened::open(owner).ok()?;
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.ok()?;
        let bytes = unsafe { read_hglobal(HGLOBAL(handle.0)) }?;
        let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        Some(String::from_utf16_lossy(&wide[..end]))
    }

    unsafe fn read_hglobal(h: HGLOBAL) -> Option<Vec<u8>> {
        unsafe {
            let size = GlobalSize(h);
            let ptr = GlobalLock(h);
            if ptr.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), size).to_vec();
            let _ = GlobalUnlock(h);
            Some(bytes)
        }
    }

    /// 复制到新的 HGLOBAL 并放进剪贴板（成功后内存归系统所有）
    unsafe fn set_bytes(format: u32, bytes: &[u8]) -> bool {
        unsafe {
            let Ok(h) = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) else { return false };
            let ptr = GlobalLock(h);
            if ptr.is_null() {
                let _ = GlobalFree(Some(h));
                return false;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(h);
            if SetClipboardData(format, Some(HANDLE(h.0))).is_err() {
                let _ = GlobalFree(Some(h));
                return false;
            }
            true
        }
    }

    /// 告诉剪贴板历史、云剪贴板和第三方剪贴板工具忽略这次内容
    unsafe fn mark_excluded() {
        unsafe {
            let zero = 0u32.to_le_bytes();
            set_bytes(RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")), &zero);
            set_bytes(RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")), &zero);
            set_bytes(RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")), &zero);
        }
    }
}

pub use imp::*;
