# Relic

自用桌面翻译工具（Windows 为主），Tauri v2 + React 19 + TypeScript + Tailwind CSS 4，AI 请求和系统交互在 Rust 侧。

- 设计文档：`docs/design.md`（功能范围、交互、架构、关键实现方案、里程碑都在里面，改动方案时同步更新它）
- 回复用中文。
- 用户偏好：极简 UI、性能、易用；对细节要求高。只做用户确认过的功能，不擅自加功能或依赖。
- 参考实现 pot-desktop：`\\wsl.localhost\Ubuntu-24.04\home\edy\code\pot-desktop`，只作参考，不照搬结构。

## 当前进度

- 设计已定稿到 v0.2。
- Windows 工具链已就绪（2026-11-09）：Rust stable-msvc（`RUSTUP_HOME`/`CARGO_HOME` 在 `D:\software\dev\`）、VS 2022 Build Tools C++ 工作负载（`D:\software\dev\VSBuildTools`）、Node + pnpm（store 在 `D:\.pnpm-store`）、WebView2。
- M0 代码已完成（2026-11-09），等用户验收视觉效果：Acrylic + 原生圆角已生效，`show_translate` 耗时约 20–35ms。
- 下一步：M0 验收后进入 M1。
