<img src="src-tauri/icons/128x128.png" width="96" alt="Relic 图标" align="right" />

# Relic

一个自用的 Windows 桌面翻译工具：划词、输入、在任意输入框里就地翻译，译文流式显示。界面极简，原生磨砂窗口，AI 请求全部在本地 Rust 进程里发出。

<p>
  <img src="docs/images/translate-light.png" width="49%" alt="翻译窗口（浅色）" />
  <img src="docs/images/translate-dark.png" width="49%" alt="翻译窗口（深色）" />
</p>

## 功能

- **划词翻译**：在任意应用里选中文字，按 `Alt+E`，翻译窗口出现在鼠标旁边并自动翻译。优先用 UI Automation 读取选区，不碰剪贴板。
- **输入翻译**：按 `Alt+F` 呼出窗口，输入后按 Enter 翻译，Shift+Enter 换行。
- **内联翻译**：在任意输入框里快速按两下 `Alt`，把内容就地替换成英文：有选中就只翻译选中部分，否则翻译整个输入框。适合在英文环境里聊天时打中文、发英文。在终端里会弹出输入框，翻译后再按 Enter 填回终端。
- **复制与回填**：复制译文，或者按 `Ctrl+Enter` 把译文粘贴回原应用（划词时替换选中内容）。临时写入的剪贴板内容不会进 Win+V 历史，完成后恢复原来的剪贴板。
- **自动方向**：原文是母语就译成第二语言，否则译成母语；点窗口左上角的方向标签可以反过来。
- **口语化译文**：默认提示词让英文译文像母语者日常说话：短句、高频词和短语动词，方便顺便学口语。提示词可以在设置里改。
- **多个 AI 服务**：支持 OpenAI 兼容（默认 DeepSeek）、Anthropic、Gemini 三种接口格式，可配置多个，随时切换。
- **界面**：浅色、深色两套渐变主题（可跟随系统），原生 Acrylic 磨砂和圆角，窗口始终置顶、高度随内容变化。托盘菜单、开机自启。

## 快捷键

| 范围 | 按键 | 作用 |
|---|---|---|
| 全局 | `Alt+E`（可改） | 划词翻译 |
| 全局 | `Alt+F`（可改） | 输入翻译 |
| 全局 | 快速按两下 `Alt` | 内联翻译（可关闭） |
| 翻译窗口 | `Enter` / `Shift+Enter` | 翻译 / 换行 |
| 翻译窗口 | `Esc` | 翻译中取消，否则隐藏窗口 |
| 翻译窗口 | `Ctrl+C` | 没有选中文字时复制译文 |
| 翻译窗口 | `Ctrl+Enter` | 回填到原应用 |

## 设置

托盘菜单 → 打开设置。改完立即生效，也可以直接编辑配置文件 `%APPDATA%\com.relic.app\config.toml`（保存后自动重新加载）。

<img src="docs/images/settings.png" width="70%" alt="设置窗口" />

## 技术栈

| 层 | 选择 |
|---|---|
| 壳 | [Tauri v2](https://tauri.app) |
| 前端 | React 19、TypeScript、Tailwind CSS 4、Vite（不用组件库和状态库） |
| 后端 | Rust：AI 请求（reqwest，SSE 流式解析）、配置（toml / toml_edit，热重载）、语言检测（whatlang + 文字系统判断） |
| 系统交互 | `windows` crate：UI Automation 取词、SendInput、低级键盘钩子、Win32 剪贴板、DWM 圆角与磨砂 |
| 前后端类型 | ts-rs 从 Rust 类型生成 TypeScript 类型 |
| 打包 | NSIS 安装包，按当前用户安装 |

只支持 Windows 11（核心功能依赖 Win32）。设计细节见 [docs/design.md](docs/design.md)。

## 开发

需要 Rust（MSVC 工具链）、Visual Studio Build Tools（"使用 C++ 的桌面开发"）、Node.js 和 pnpm。

```bash
pnpm install
```

```bash
pnpm tauri dev
```

打包，生成 `src-tauri/target/release/bundle/nsis/Relic_<版本>_x64-setup.exe`：

```bash
pnpm tauri build
```

Rust 单元测试（同时更新 `src/lib/bindings/` 下的 TS 类型）：

```bash
cd src-tauri && cargo test
```
