# Relic

自用桌面翻译工具（Windows 为主），Tauri v2 + React 19 + TypeScript + Tailwind CSS 4，AI 请求和系统交互在 Rust 侧。

- 设计文档：`docs/design.md`（功能范围、交互、架构、关键实现方案、版本记录都在里面，改动方案时同步更新它）
- 回复用中文。
- 用户偏好：极简 UI、性能、易用；对细节要求高。只做用户确认过的功能，不擅自加功能或依赖。
- 参考实现 pot-desktop：`\\wsl.localhost\Ubuntu-24.04\home\edy\code\pot-desktop`，只作参考，不照搬结构。

## 当前状态

- MVP（M0–M4）已完成，已发布 v0.1.0、v0.1.1（GitHub Releases，私有仓库 ElmTran/relic）。后续（P1）候选见设计文档第 2 节。
- 用户用的是国内自定义 OpenAI 兼容服务（base_url 需带 `/v1`）。
- 用户实测过的场景：记事本、Chrome、微信、PowerShell、WSL。改动划词、回填、内联翻译时要考虑这些应用。

## 开发

- 工具链在 `D:\software\dev\`（Rust stable-msvc、VS 2022 Build Tools）。PowerShell 会话里要先设环境变量才能用 cargo：
  `$env:RUSTUP_HOME='D:\software\dev\rustup'; $env:CARGO_HOME='D:\software\dev\cargo'; $env:PATH="D:\software\dev\cargo\bin;$env:PATH"`
- 运行：`pnpm tauri dev`。Rust 测试：`cd src-tauri && cargo test`（同时更新 `src/lib/bindings/` 的 TS 类型）。前端类型检查：`pnpm exec tsc --noEmit`。
- 打包和发版步骤见设计文档第 6 节。
