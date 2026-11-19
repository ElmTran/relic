//! 常驻窗口隐藏时让 WebView2 休眠：调低内存目标并挂起渲染进程，系统会把它的内存页换出 RAM。
//! 显示前唤醒。前端隐藏时没有要跑的任务（请求、热键都在 Rust 侧），挂起不影响功能。

/// awake=false 必须在窗口隐藏之后调用，awake=true 要在显示窗口、向页面发事件之前调用
/// （挂起的页面收不到事件，要等恢复后才处理）。失败只记日志
#[cfg(windows)]
pub fn set_awake(win: &tauri::WebviewWindow, awake: bool) {
    let label = win.label().to_string();
    let label2 = label.clone();
    let result = win.with_webview(move |webview| {
        if let Err(e) = unsafe { apply(&webview.controller(), awake, &label) } {
            eprintln!("[relic] {label} WebView {}失败: {e}", if awake { "唤醒" } else { "休眠" });
        }
    });
    if let Err(e) = result {
        eprintln!("[relic] {label2} 取不到 WebView: {e}");
    }
}

#[cfg(windows)]
unsafe fn apply(
    controller: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
    awake: bool,
    label: &str,
) -> windows_core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_3,
        ICoreWebView2_19,
    };
    use webview2_com::TrySuspendCompletedHandler;
    use windows_core::Interface;

    unsafe {
        // 可见的 WebView 不能挂起；变为可见时会自动恢复
        controller.SetIsVisible(awake)?;
        let core = controller.CoreWebView2()?;

        // 内存目标是 Runtime 109+ 的接口，没有就跳过，不影响挂起
        let level = if awake {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        };
        if let Err(e) = core.cast::<ICoreWebView2_19>().and_then(|c| c.SetMemoryUsageTargetLevel(level)) {
            eprintln!("[relic] {label} 设置内存目标失败: {e}");
        }

        let core3: ICoreWebView2_3 = core.cast()?;
        if awake {
            core3.Resume()?;
        } else {
            let label = label.to_string();
            let handler = TrySuspendCompletedHandler::create(Box::new(move |result, suspended| {
                match result {
                    Err(e) => eprintln!("[relic] {label} WebView 挂起失败: {e}"),
                    Ok(()) if !suspended => eprintln!("[relic] {label} WebView 未挂起"),
                    #[cfg(debug_assertions)]
                    Ok(()) => eprintln!("[relic] {label} WebView 已挂起"),
                    #[cfg(not(debug_assertions))]
                    Ok(()) => {}
                }
                Ok(())
            }));
            core3.TrySuspend(&handler)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_awake(_win: &tauri::WebviewWindow, _awake: bool) {}
