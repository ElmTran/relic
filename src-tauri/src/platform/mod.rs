//! 平台相关代码。系统交互只在 Windows 下实现；其他平台只保留窗口效果、打开文件的空实现，
//! 划词和回填在编排层直接报"不支持"。

#[cfg(windows)]
pub mod clipboard;
pub mod effects;
#[cfg(windows)]
pub mod focus;
#[cfg(windows)]
pub mod input;
#[cfg(windows)]
pub mod keyhook;
#[cfg(windows)]
pub mod selection;
pub mod shell;
