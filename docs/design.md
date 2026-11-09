# Relic 设计文档

> 状态：草案 v0.2（2026-11-09）
> Relic 是一个自用的桌面翻译工具，交互参考 pot-desktop，但从零重写。原则：**简洁、快、顺手**。

## 1. 定位与原则

- **主平台 Windows 11**。代码保持可移植：平台相关部分全部隔离在 `platform/` 模块，其他平台先留空实现。
- **只接 AI 翻译**，不做插件系统，也不接传统翻译服务商。
- **窗口常驻**：启动时创建，之后只做显示和隐藏，切换到别的应用不会关闭。
- **UI 极简**：半透明磨砂、圆角，支持浅色和深色主题。

## 2. 功能范围

### MVP

| 功能 | 说明 |
|---|---|
| 划词翻译 | 在任意应用选中文本后按热键，弹出翻译窗口并流式显示译文 |
| 输入翻译 | 按热键弹出窗口，输入框聚焦，按 Enter 翻译 |
| 语言检测与自动反向 | 原文是母语（中文）就译成第二语言（英文），否则译成母语；可在窗口里手动改方向 |
| 复制与回填 | 复制译文；或者把译文**粘贴回原应用**（划词时替换选中内容，输入翻译时插入到光标处） |
| 双击空格内联翻译 | 在任意输入框里连按两次空格，把当前内容就地替换成英文 |
| AI 服务 | 支持 OpenAI、Anthropic、Gemini 三种 API 格式，可配置多个，同一时间使用其中一个；默认 DeepSeek |
| 托盘 | 显示窗口、切换主题、打开设置、退出 |
| 主题 | 跟随系统、浅色、深色三种 |

### 后续（P1）

历史记录（SQLite 全文搜索）、截图 OCR 和截图翻译（Windows.Media.Ocr 或 LLM 视觉）、TTS、提示词预设（润色、解释）、开机自启、命令行调用。

### 不做

插件系统、多翻译源并排对比、WebDAV/云备份（配置文件本身就可以同步）、自动更新、界面多语言。

## 3. 交互设计

### 3.1 翻译窗口

```
╭────────────────────────────────────╮
│ 中文 → English                 ✕ │  ← 整行可拖拽，图标平时淡显；方向标签可点击切换
│ 原文（可编辑，默认最多显示 3 行）       │
│ ────────────────────────────────── │  ← 1px 细分隔线
│ 译文（流式输出）                       │
│                                    │
│                       ⧉ 复制  ↩ 回填 │  ← 鼠标悬停时显示
╰────────────────────────────────────╯
```

- 高度随内容自动变化，上限为显示器工作区高度的 60%，超过后译文区滚动；窗口底部不会超出工作区。宽度可以拖动调整，并记住（目前只在本次运行内记住，持久化待定）。
- 方向标签：翻译前显示"自动检测"；翻译后显示"原文语言 → 目标语言"，点击切换为另一个方向，并重新翻译。改了原文再按 Enter，又回到自动方向。
- 翻译失败时，在译文区显示错误原因和"重试"按钮。
- 复制、回填按钮在有完整译文时才可用；复制成功、回填失败的原因在底栏左侧提示 2 秒。
- **窗口行为**
  - 划词翻译：出现在鼠标附近，限制在当前显示器工作区内。
  - 输入翻译：出现在当前显示器中央。
  - 窗口已经显示时再次触发，只替换内容，不移动位置。
  - 隐藏窗口的方式：Esc、✕ 按钮、托盘菜单。**失焦不隐藏**。设置里提供"失焦时隐藏"选项，默认关闭。
  - **始终置顶**，切到别的窗口也不会被盖住。因此不设 📌 固定按钮。

### 3.2 快捷键

| 范围 | 按键 | 作用 |
|---|---|---|
| 全局 | `Alt+E`（可改） | 划词翻译 |
| 全局 | `Alt+F`（可改） | 输入翻译 |
| 全局 | 连按两次空格 | 内联翻译（可关闭） |
| 窗口内 | `Esc` | 正在翻译时取消，否则隐藏窗口 |
| 窗口内 | `Enter` / `Shift+Enter` | 输入模式下：翻译 / 换行 |
| 窗口内 | `Ctrl+C` | 窗口内没有选中文本时，复制译文 |
| 窗口内 | `Ctrl+Enter` | 回填：粘贴回原应用 |

### 3.3 视觉风格

- 原生 Acrylic 磨砂背景，上面叠一层半透明底色和柔和的多色径向渐变（参考 Fluent Search 的渐变窗口，但只做浅色、深色各一套）：浅色是左上淡蓝、右下淡粉、左下一点青；深色是左上深蓝、右下紫、左下一点青。1px 细边框；不使用厚重阴影。
- 浅色底色不透明度较高（约 0.72）：窗口后面是深色内容时，透明度太高会让整体发灰。
- 字体用 `Segoe UI Variable` 和 `Microsoft YaHei UI`，字号 13 到 14px，留白充足。
- 图标用 lucide；动画只用 CSS 过渡，不引入动画库。
- 主题通过 CSS 变量实现，同时调用 `window.set_theme()` 让原生磨砂的色调跟着变。

## 4. 技术架构

### 4.1 技术栈

| 层 | 选择 | 说明 |
|---|---|---|
| 壳 | Tauri v2 | |
| 前端 | React 19、TypeScript、Tailwind CSS 4、Vite | 不用组件库，也不用状态库 |
| 系统层 | Rust | 取词、回填、键盘钩子、磨砂效果、AI 请求 |
| Tauri 插件 | global-shortcut、single-instance、autostart | 托盘用 Tauri 内置功能；剪贴板自己用 Win32 实现，见 5.4 |
| Win32 | `windows` crate | UIA、SendInput、键盘钩子、DWM、剪贴板 |
| 前后端类型 | `ts-rs` | IPC 只有十来个命令，从 Rust 类型生成 TS 类型就够了 |

**为什么 AI 请求放在 Rust：** 双击空格内联翻译时没有可见窗口，整个流程（取文本、翻译、回填）都在 Rust 里完成。翻译窗口复用同一个客户端，所以只需要实现一份。这样还有几个好处：API key 不进入前端，不需要绕过 CORS，可以单独测试。

### 4.2 分层

```
┌──────────── 前端 WebView（只负责显示和交互）──────────────┐
│  translate 窗口（常驻） · settings 窗口（按需） · indicator 浮标 │
└──── invoke 命令 / Channel 流式推送 / event（显示、配置变更）────┘
┌─────────────────────────── Rust ───────────────────────────┐
│ 触发    热键 · 托盘 · 键盘钩子（双击空格）· 单实例参数              │
│ 编排    show_translate / paste_back / inline_translate          │
│ ai      Provider 适配器 · SSE 流解析 · 取消 · 提示词模板             │
│ 基础    config.toml（热重载）· 语言检测 · 日志                      │
│ 平台    选中文本 · 剪贴板 · 输入模拟 · 焦点 · 窗口效果                │
└────────────────────────────────────────────────────────────┘
```

### 4.3 目录结构

```
relic/
├─ docs/
├─ src/                          # 前端
│  ├─ windows/
│  │  ├─ translate/              # 每个窗口单独一个 Vite 入口，只加载自己需要的代码
│  │  ├─ settings/
│  │  └─ indicator/              # 双击空格时显示的"翻译中…"小浮标
│  ├─ components/
│  ├─ lib/ipc.ts                 # 命令封装和生成的类型
│  └─ styles/theme.css
└─ src-tauri/src/
   ├─ lib.rs                     # 组装：插件、窗口、托盘、热键
   ├─ config.rs
   ├─ lang.rs
   ├─ orchestrator.rs            # 各个功能的完整流程
   ├─ ui.rs                      # 窗口创建、定位、显示和隐藏
   ├─ tray.rs
   ├─ ai/
   │  ├─ mod.rs                  # Provider trait、调度、取消
   │  ├─ openai.rs  anthropic.rs  gemini.rs
   │  ├─ sse.rs
   │  └─ prompt.rs
   └─ platform/                  # 只在 windows 下编译，其他平台为空实现
      ├─ selection.rs            # 获取选中文本：UIA 优先，失败时回退到 Ctrl+C
      ├─ clipboard.rs            # 快照、恢复、不写入剪贴板历史
      ├─ input.rs                # SendInput，等待修饰键松开
      ├─ focus.rs                # 记录和恢复前台窗口
      ├─ keyhook.rs              # 低级键盘钩子 WH_KEYBOARD_LL
      └─ effects.rs              # 圆角、Acrylic
```

### 4.4 IPC

- **命令**
  - `translate(req, channel)`：结果通过 Channel 推送，事件依次为 `Started → Delta* → Done | Error`。`Started` 带方向信息，`Done` 带后处理后的完整译文，前端用它替换流式拼出的内容
  - `cancel`
  - `hide`
  - `get_theme`
  - `resize_to_content`
  - M2：`copy_text`、`paste_back`
  - M4（设置窗口）：`get_config`、`set_config`
- **事件**
  - `relic://show`：载荷为 `{ mode: "selection" | "input", text }`，`text` 为 null 时保留窗口现有内容
  - `relic://theme`：解析后的主题 `light | dark`
  - `relic://config-changed`
- 前后端共用的类型由 ts-rs 生成到 `src/lib/bindings/`，在 `src-tauri` 下运行 `cargo test` 时更新。

## 5. 关键实现方案

### 5.1 窗口与磨砂

- 窗口参数：`decorations: false`、`transparent: true`、`skipTaskbar: true`、`alwaysOnTop: true`、`shadow: false`、`windowEffects: [acrylic]`。
- 关掉 `shadow`：DWM 的窗口阴影大小由系统固定，激活时很宽，不能调，只能关。关掉后原生圆角和 1px 细边框仍然保留，还会剩一圈很淡的阴影。
- 启动时创建，设为隐藏；之后只调用 `show`、`hide`、`set_position`。目标：从按热键到窗口显示在 100ms 以内。
- 在 Win11 22H2 及以上版本，Acrylic 由 DWM 的系统背景（`DWMSBT_TRANSIENTWINDOW`）实现，拖动时不卡。本机是 26100，满足条件。
- **圆角（已定）**：用原生圆角，调用 `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND)`，半径由系统固定（约 8px），和真实磨砂配合。不用 CSS `border-radius` 自己画圆角，因为 Acrylic 会铺满整个矩形窗口，圆角外面也会有磨砂。1px 细边框也由 DWM 原生绘制，CSS 不再画边框。
- 注意：CSS 的 `backdrop-filter` 只能模糊 WebView 内部的内容，模糊不到窗口后面的桌面，所以磨砂必须靠原生 Acrylic。

### 5.2 获取选中文本（划词）

1. 按热键时，先记录前台窗口的 HWND，供后面回填使用（前台是 Relic 自己时不记录）。取词有等待，放到独立线程，不卡事件循环。
2. **Alt 热键的副作用**：热键吃掉了 `E`，原应用只看到 Alt 按下又松开，会激活菜单栏。热键触发时如果 Alt 还按着，就注入一个无用的虚拟键 `0xE8`（AutoHotkey 的"菜单屏蔽键"），Alt 松开就不算单击。
3. **优先用 UI Automation**：取焦点元素的 `TextPattern`，调用 `GetSelection`。这种方式不碰剪贴板，原生控件、Chromium 和 Office 都支持。UIA 调用在单独的线程里跑，超过 300ms 就当作不支持。
   - UIA 能用、只是选区为空时，以 UIA 为准，**不回退**：VS Code 等编辑器没选中时按 Ctrl+C 会复制整行。
4. **UIA 不支持时回退到 Ctrl+C**：
   - 先给剪贴板拍快照；
   - **等待热键的修饰键松开**（最多 1 秒），否则 Alt 还按着，发出去的就变成 Ctrl+Alt+C；
   - 用 SendInput 发送 Ctrl+C；
   - 等待 `GetClipboardSequenceNumber` 变化，超时 300ms（没变化说明没选中，不用恢复）；
   - 读取文本，然后恢复剪贴板快照。
5. 取到文字就在鼠标右下方显示窗口并自动翻译（放不下时翻到上方、左侧，限制在工作区内）；什么都没取到就按输入翻译处理。
6. 实测（2026-11-09）：UIA 约 4–9ms；回退到 Ctrl+C 约 135–220ms，大部分是在等用户松开 Alt。

### 5.3 回填（复制并粘贴回原应用）

1. 给剪贴板拍快照，把译文写入剪贴板。写入时附带 `ExcludeClipboardContentFromMonitorProcessing`、`CanIncludeInClipboardHistory=0`、`CanUploadToCloudClipboard=0`，这样临时内容不会出现在 Win+V 的剪贴板历史里。
2. 趁 Relic 自己还是前台，调用 `SetForegroundWindow` 切回之前记录的 HWND（最小化的先还原），等焦点真正生效（最多 500ms），再隐藏 Relic 窗口。
3. 等修饰键松开（Ctrl+Enter 回填时 Ctrl 可能还按着），发送 Ctrl+V。
4. 等 300ms，让目标程序读完剪贴板，再恢复成原来的内容（恢复的内容同样带排除格式，避免历史里出现重复项）。配置 `keep_result_in_clipboard = true` 时不恢复，译文作为正常内容留在剪贴板里。
5. 划词时原应用里的选区还在，所以 Ctrl+V 会替换选中内容；输入翻译时插入到光标处。两种情况是同一套流程。
6. **复制按钮**：正常写入剪贴板，会进 Win+V 历史，因为这是用户主动复制。

### 5.4 剪贴板

不用 Tauri 的剪贴板插件，自己用 Win32 实现，因为需要：

- **快照和恢复**：保留所有基于 HGLOBAL 的格式，不只是文本；跳过 GDI 句柄（位图、图元文件、调色板）和私有句柄格式。CF_BITMAP 等会由系统从 CF_DIB 自动合成；
- **不写入剪贴板历史**；
- 用**序列号**判断剪贴板是否变化；
- 剪贴板可能正被别的程序占用，打开时重试最多 300ms；以翻译窗口的 HWND 作为所有者。
- 注入的按键都在 `dwExtraInfo` 里带标记 `RELI`，M3 的键盘钩子据此忽略自己发出的键。

### 5.5 双击空格内联翻译

- **触发**
  - 在独立线程里安装 `WH_KEYBOARD_LL` 钩子并运行消息循环。
  - 忽略 Relic 自己注入的按键（带 `LLKHF_INJECTED` 标志）。
  - 两次空格间隔小于 `interval_ms`（默认 300ms）就触发；中间按了其他键则重新计数。
  - 钩子**不拦截**任何按键，只做观察。
- **流程**
  1. 记录当前 HWND，检查进程是否在黑名单里。
  2. 在光标附近显示"翻译中…"浮标。光标位置用 `GetGUIThreadInfo` 取，拿不到就用鼠标位置。
  3. 发送两次 Backspace，删掉刚输入的两个空格。
  4. 确定要翻译的内容：先按 5.2 的方式读取选中文本；**有选中就只翻译选中的部分**，没有选中就发送 Ctrl+A 选中整个输入框再复制。
  5. 用剪贴板方式复制文本。
  6. 调用 AI（不流式，使用 `inline` 提示词）。
  7. 确认前台窗口仍然是第 1 步记录的那个，然后粘贴，再恢复剪贴板。
- 翻译过程中按 Esc 可以取消。如果检测到原文已经是目标语言，就什么都不做。
- **风险**：中文输入法用空格选词。刚上屏又紧接着按空格，可能被误判为双击。应对办法：
  - 触发方式可配置（双击或三击空格，间隔可调）；
  - 应用黑名单；
  - 可以通过 `ImmGetDefaultIMEWnd` 加 `WM_IME_CONTROL` 跨进程查询输入法当前是否处于中文模式，作为辅助判断。
  - 原型阶段实测后再决定默认值。

### 5.6 AI 服务

- **Provider 适配器**：每种 API 格式只负责三件事：
  - 构造请求，包括鉴权头和请求体；
  - 解析流式事件里的增量文本；
  - 解析非流式响应。
  - SSE 解析、超时、取消这些是公共逻辑。
- **取消**：发起新请求时中止还在进行的请求（对 tokio 任务调用 abort）。前端给每次请求编号，只处理最新一次的事件。
- **后处理**：去掉模型自己加的首尾引号或代码块标记（原文本身就带的不去）。
- **提示词**：系统提示词只含 `{from}` `{to}` 两个变量，原文总是作为最后一条用户消息单独发送。这样系统提示词基本固定，可以命中 DeepSeek 自动的上下文缓存（Context Caching on Disk），降低延迟和费用。
- **网络**：默认跟随系统代理；连接超时 5 秒；不自动重试，界面上提供重试按钮。HTTP 客户端全局复用，保持连接以降低延迟；TLS 用 Windows 自带的 schannel；带 `User-Agent: Relic/<版本>`（有的服务对没有 UA 的请求直接 403）。
- **地址**：请求地址是 `base_url` + `/chat/completions`。DeepSeek 官方不需要 `/v1`，但多数兼容服务的 `base_url` 要以 `/v1` 结尾，否则路径不存在，可能返回 403 而不是 404。
- **DeepSeek（2026-11 文档）**：当前模型名为 `deepseek-flash`（旧的 `deepseek-chat` 已下线）。所有模型**默认开启思考模式**，开启时 `temperature` 无效、延迟高，所以默认配置用 `extra_body` 关闭：`{ thinking = { type = "disabled" } }`。`extra_body` 是通用字段，会原样合并进请求体顶层。

### 5.7 语言检测

- 先按字符所属的文字系统判断：有假名就是日文，有谚文就是韩文；有汉字时，一个汉字按半个拉丁单词算，汉字多就是中文（中英混排按这个比例判断）。
- 其他情况交给 `whatlang`（体积小、速度快），不用 lingua，因为它的模型太大。拉丁字母的短文本（少于 5 个单词）whatlang 不可靠时按英文处理。
- 翻译方向：检测结果等于母语就译成第二语言，否则译成母语。

### 5.8 配置

- 位置：`%APPDATA%/com.relic.app/config.toml`。首次启动写出带注释的模板（单元测试保证模板和代码里的默认值一致）。
- Rust 里有一个带类型的 `Config` 结构体，加了 `#[serde(default)]`，缺少的字段自动用默认值。Rust 是配置的唯一来源，前端只能通过命令读写。
- 监听配置文件变化（监听所在目录，兼容"写临时文件再改名"的编辑器，防抖 150ms）：手动改文件也会热重载，主题、热键立即生效，并广播 `config-changed`。文件写错时保留当前配置；启动时就写错则用默认值，但不覆盖用户的文件。
- 程序自己改配置（如托盘切换主题）时用 `toml_edit` 只改对应的键，保留用户的注释和格式。
- 设置窗口做出来之前（M4），托盘的"打开设置"直接用默认程序打开 config.toml，没有关联程序时用记事本。
- API key 先明文存在配置文件里（只有当前用户能访问的目录）。以后可以改成存到 Windows 凭据管理器（`keyring` crate）。

```toml
[general]
theme = "system"            # system | light | dark
primary_lang = "zh"
secondary_lang = "en"
hide_on_blur = false
keep_result_in_clipboard = false

[hotkeys]
selection = "Alt+E"
input = "Alt+F"

[inline]
enabled = true
trigger = "double_space"    # double_space | triple_space
interval_ms = 300
target_lang = "en"
scope = "selection_or_all"  # 有选中翻译选中部分，否则翻译整个输入框
blacklist = ["Code.exe", "WINWORD.EXE"]

[ai]
active = "deepseek"

[[ai.providers]]
id = "deepseek"
format = "openai"           # openai | anthropic | gemini
base_url = "https://api.deepseek.com"
api_key = "sk-..."
model = "deepseek-flash"
temperature = 1.3           # DeepSeek 文档对翻译场景的推荐值
stream = true
extra_body = { thinking = { type = "disabled" } }   # 原样合并进请求体

[prompts]
translate = """..."""       # 系统提示词，可用变量：{from} {to}；原文作为用户消息单独发送
inline = """..."""
```

## 6. 构建与开发环境

- Tauri 的跨平台指的是**同一份源码可以在各个平台构建**，安装包一般要在对应系统上打。以后需要多平台时用 GitHub Actions 矩阵构建即可。从 Linux 交叉编译 Windows 官方标注为实验性，不采用。
- 本项目的核心功能（取词、回填、键盘钩子、磨砂）都依赖 Win32，所以**开发和调试必须在 Windows 上进行**。
- 仓库位置：`D:\code\relic`，在 Windows 原生环境下开发（Claude Desktop 的 Windows 会话）。
- Windows 侧工具链（2026-11-09 已装好）：Rust stable-msvc（`RUSTUP_HOME`、`CARGO_HOME` 在 `D:\software\dev\`）、VS 2022 Build Tools C++ 工作负载（`D:\software\dev\VSBuildTools`）、Node 26 + pnpm（store 在 `D:\.pnpm-store`）、WebView2。Windows SDK 和 VS Installer 只能装在 C 盘。
- 开发：`pnpm tauri dev`。前端改动热更新；Rust 改动会自动重新编译并重启应用。
- 参考实现 pot-desktop 在 WSL 里：`\\wsl.localhost\Ubuntu-24.04\home\edy\code\pot-desktop`，只作参考，不照搬。

## 7. 里程碑

| 阶段 | 内容 | 验收标准 |
|---|---|---|
| M0 原型 | 搭好 Tauri v2 + React 脚手架；做出翻译窗口的磨砂、圆角、主题切换；窗口常驻。为了能触发显示和切换主题，提前做了精简托盘（显示窗口、主题、退出）和写死的 `Alt+W`（M1 起热键从配置读取） | 视觉效果满意；显示延迟 <100ms；拖动不卡 |
| M1 翻译核心 | config.toml、OpenAI 格式适配器（流式和取消）、语言检测、输入翻译、托盘（补设置入口，热键改为从配置读取） | 可以用 DeepSeek 完成输入翻译 |
| M2 划词与回填 | 全局热键、获取选中文本、焦点记录与恢复、剪贴板快照、复制和回填 | 在浏览器、微信、记事本中都能正常划词和回填 |
| M3 内联翻译 | 键盘钩子、浮标、黑名单 | 双击空格可用，打中文时没有误触发 |
| M4 完善 | 设置窗口、Anthropic 和 Gemini 适配器、开机自启、打包 | 日常可用 |

## 8. 已定决策

- 双击空格的翻译范围：有选中文字就只翻译选中部分，没有选中就翻译整个输入框。
- 圆角：原生圆角（系统固定约 8px）配真实 Acrylic 磨砂。
- 开发环境：仓库放在 `D:\code\relic`，在 Windows 原生环境下开发。

## 9. 待定问题

1. ~~热键默认值~~ 已定：`Alt+E` 划词、`Alt+F` 输入（沿用 pot 的习惯；`Alt+Q` 被 Fluent Search 占用）。
2. 双击空格的默认触发方式和间隔，等 M3 原型实测输入法误触发情况后再定。
