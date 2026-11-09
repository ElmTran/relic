//! 获取其他应用里选中的文本：UI Automation 优先（不碰剪贴板），不支持时回退到 Ctrl+C。

use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId};

use super::{clipboard, input};

/// UIA 调用可能卡在目标进程里，超过这个时间就当作不支持
const UIA_TIMEOUT: Duration = Duration::from_millis(300);
/// Ctrl+C 之后等剪贴板变化的时间
const COPY_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug)]
pub enum Via {
    Uia,
    Clipboard,
}

/// 返回选中的文本（可能为空）和取到它的方式
pub fn get(owner: isize) -> (String, Via) {
    match uia_with_timeout() {
        // UIA 能用：有没有选中都以它为准。没选中时不回退，
        // 因为 VS Code 等编辑器没选中时按 Ctrl+C 会复制整行
        Some(text) => (text, Via::Uia),
        None => (via_clipboard(owner).unwrap_or_default(), Via::Clipboard),
    }
}

/// Some(text)：焦点元素支持 TextPattern（text 可能为空）；None：不支持、出错或超时
fn uia_with_timeout() -> Option<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(uia_selection());
    });
    rx.recv_timeout(UIA_TIMEOUT).ok().flatten()
}

fn uia_selection() -> Option<String> {
    unsafe {
        let inited = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = (|| -> windows::core::Result<String> {
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let element = uia.GetFocusedElement()?;
            let pattern: IUIAutomationTextPattern = element.GetCurrentPatternAs(UIA_TextPatternId)?;
            let ranges = pattern.GetSelection()?;
            let mut text = String::new();
            for i in 0..ranges.Length()? {
                text.push_str(&ranges.GetElement(i)?.GetText(-1)?.to_string());
            }
            Ok(text)
        })();
        if inited {
            CoUninitialize();
        }
        result.ok()
    }
}

/// 回退方案：快照剪贴板 → 等修饰键松开 → Ctrl+C → 等序列号变化 → 读文本 → 恢复快照
fn via_clipboard(owner: isize) -> Option<String> {
    let snapshot = clipboard::snapshot(owner).ok();
    input::wait_modifiers_released(Duration::from_secs(1));

    let before = clipboard::sequence();
    input::ctrl_c();
    let start = Instant::now();
    let changed = loop {
        if clipboard::sequence() != before {
            break true;
        }
        if start.elapsed() > COPY_TIMEOUT {
            break false;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if !changed {
        return None; // 没有选中内容，剪贴板没动，不用恢复
    }
    // 目标程序可能分几次写入，稍等一下再读
    std::thread::sleep(Duration::from_millis(20));
    let text = clipboard::get_text(owner);
    if let Some(snapshot) = snapshot {
        let _ = clipboard::restore(owner, &snapshot);
    }
    text
}
