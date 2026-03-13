# Eidolon-Echo

一个以 AI 对话、角色扮演与长期陪伴为核心的桌面应用。

- 前端：Vite + React（由 Tauri 加载）
- 桌面壳：Tauri + Rust
- 后端：Rust（Axum）
- 数据库：SQLite（`sqlx` migration）
- AI 协议：OpenAI-Compatible（默认 DeepSeek）

当前是前后端分离架构：

- `apps/backend` 提供 HTTP API，可独立运行/替换
- `apps/desktop/web` 只依赖 HTTP API
- `apps/desktop/src-tauri` 只负责窗口管理、sidecar 生命周期与桌面能力

平台状态：

- 当前开发与日常验证环境为 macOS。
- 项目结构本身按跨平台方向组织，但目前未对 Windows / Linux 做完整验证。
- 如果你在非 macOS 环境使用，请默认按“实验状态”看待，并预期可能需要额外适配。


开发手册见：[DEVELOPMENT.md](./DEVELOPMENT.md)。

## 当前功能说明

- AI 双模式：
  - `default`：轻量对话，消息只保存在内存缓存，不写数据库消息表。
  - `roleplay`：带记忆对话，消息会写入数据库，并在冷启动/翻页时回填缓存。
- 会话与历史：
  - 按 `mode -> profile.active_conversation_id -> conversation` 解析当前会话。
  - 历史接口：`GET /api/messages?mode=...&limit=...&before_id=...`。
  - 历史列表最新消息在上，支持滚动分页加载更早消息。
- 上下文拼接：
  - 系统提示词使用 `profiles.system_prompt`（数据库配置）。
  - 上下文历史优先从内存缓存读取，不足时再从数据库补齐。
  - 缓存键按 `conversation_id` 管理，避免多会话串上下文。
- 设置中心：
  - 支持 provider（当前主用 DeepSeek）与 profile（default/roleplay）配置。
  - 可配置温度、max_tokens、头像路径、提示词、context_limit。
  - 设置页分为 `概览 / API 设置 / 默认模式 / 角色扮演 / 其他` 五个页面。
  - `其他` 页提供本地数据清理入口。
  - 配置保存后写入数据库，后续请求按数据库配置生效。
- 桌面端交互：
  - 托盘支持模式切换、设置中心、最小化/恢复、退出。
  - 多窗口模型：`main/chat/bubble/menu/settings`，窗口职责分离。

## 各目录功能说明

- `apps/backend`：后端服务（Axum + SQLite + AI client）
  - `config/`：默认运行配置（服务端口、provider、缓存参数等）
  - `migrations/`：数据库 schema 与结构迁移
  - `openapi/`：OpenAPI 描述
  - `docs/`：面向开发者的接口文档
  - `src/ai/`：AI 抽象层、OpenAI-Compatible 客户端、上下文缓存管理
  - `src/db/`：按资源拆分的数据访问层（conversations/messages/profiles/providers）
  - `src/handlers/`：HTTP handler 层（chat、profiles、providers、health）
  - `src/main.rs`：后端入口，组装配置、路由、状态
- `apps/desktop/src-tauri`：桌面壳与窗口生命周期管理
  - `src/desktop_pet/`：桌宠域逻辑（命令、状态、窗口管理、托盘菜单）
  - `src/overlay/`：平台相关窗口行为（macOS/windows/fallback）
  - `capabilities/`：Tauri 权限能力声明
- `apps/desktop/web`：前端页面与交互逻辑（React + Vite）
  - `src/api/`：后端 API 调用封装
  - `src/*-main.jsx`：各窗口入口（main/chat/bubble/menu/settings）
  - `*.html`：多窗口页面挂载点
- `apps/backend/data`：本地 SQLite 数据库文件目录（运行时生成）


## 工作区结构（当前）

```text
.
├── Cargo.toml                      # Rust workspace
├── apps
│   ├── backend
│   │   ├── config/default.toml
│   │   ├── docs/http-api.md
│   │   ├── migrations/0001_init.sql
│   │   ├── openapi/openapi.yaml
│   │   └── src
│   │       ├── ai/
│   │       ├── config.rs
│   │       ├── db/
│   │       ├── db.rs
│   │       ├── handlers/
│   │       └── main.rs
│   └── desktop
│       ├── src-tauri
│       │   ├── capabilities/default.json
│       │   ├── tauri.conf.json
│       │   └── src
│       │       ├── desktop_pet
│       │       │   ├── commands
│       │       │   │   ├── avatar.rs
│       │       │   │   ├── chat.rs
│       │       │   │   ├── menu.rs
│       │       │   │   ├── settings.rs
│       │       │   │   └── overlay.rs
│       │       │   ├── state.rs
│       │       │   ├── window_manager.rs
│       │       │   └── mod.rs
│       │       ├── overlay
│       │       │   ├── macos.rs
│       │       │   ├── windows.rs
│       │       │   ├── fallback.rs
│       │       │   └── mod.rs
│       │       └── main.rs
│       └── web
│           ├── index.html          # avatar window
│           ├── chat.html           # input window
│           ├── bubble.html         # reply bubble window
│           ├── menu.html           # menu/history window
│           ├── settings.html       # settings center window
│           ├── src/
│           └── vite.config.js
└── README.md
```

## 结构评估

评估时间：当前版本

### 优点

- 分层清晰：`backend` / `web` / `src-tauri` 职责边界明确。
- 桌面域模型可读性较好：`desktop_pet` 里按 `commands + state + window_manager` 组织。
- 平台能力隔离正确：`overlay/mod.rs` 通过 `cfg` 分发到 `macos/windows/fallback`。
- 多窗口职责清楚：`main(形象) + chat(输入) + bubble(回复) + menu(菜单)`。

### 当前主要风险

- 窗口标签字符串分散在多个文件（`"main"`, `"chat"`, `"bubble"`, `"menu"`），后续改名风险较高。
- `overlay/macos.rs` 已承载较多职责（panel 初始化、层级切换、child 绑定、日志），复杂度上升较快。
- 自动化验证不足：目前主要靠 `cargo check` / 手工运行，没有窗口行为回归测试。
- `gen/schemas/*` 与手动改动并存，后续容易出现“配置改了但 schema 未同步”的偏差。

### 建议优先级

1. 抽取窗口常量（统一放在 `desktop_pet/window_labels.rs` 或 `overlay/constants.rs`）。
2. 给 `overlay/macos.rs` 拆分职责（初始化、空间行为、层级控制、child attachment）。
3. 增加最小可回归检查脚本（启动后检查窗口存在性、配置加载、关键命令可调用）。
4. 持续完善 `README` 的“故障排查”章节，按新增问题补充日志关键词与定位方式。

## 运行说明

### 1) 环境准备

1. 安装 Rust 工具链

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

2. 安装 Node.js（用于前端和 Tauri 前置命令）
3. 安装 Tauri CLI（若未安装）

```bash
cargo install tauri-cli --version '^2'
```

4. macOS 需要 Xcode Command Line Tools

```bash
xcode-select --install
```

### 2) 启动桌面端（开发）

```bash
cd apps/desktop/src-tauri
cargo tauri dev
```

说明：

- 首次运行前先在 `apps/desktop/web` 执行一次 `npm install`。
- `cargo tauri dev` 会自动拉起后端 sidecar（默认 `127.0.0.1:3001`）。
- 建议固定在 `apps/desktop/src-tauri` 目录执行 `cargo tauri dev`。
- 托盘 `Quit` 或应用退出事件会尝试结束 sidecar 后端进程。
- 开发模式下后端数据库路径为 `apps/backend/data/chat.db`。

### 3) 配置 AI Provider（推荐在设置中心）

请在设置中心的 API 页填写 provider 字段（会写入数据库）：

- `base_url`
- `model_name`
- `api_key`（这里直接填真实 API key 字符串，会明文存到本地数据库）
- `temperature`、`max_tokens`（可选）

说明：

- 系统会在后端启动时做一次 key 预检查，并在 `GET /api/health` 的 `ai_precheck` 返回结果。
- 默认配置文件是 `apps/backend/config/default.toml`，主要用于初始值与无数据库配置时兜底。
- 数据库迁移会在启动时自动执行（`sqlx` migrator）。
- `max_tokens` 建议填正整数；留空表示不限制。

### 4) 独立启动后端（可选）

仅在你需要单独调试后端时使用：

```bash
cargo run -p eidolon-echo-backend
```

常用环境变量覆盖（其余配置不再走 ENV fallback）：

- `APP_CONFIG`：指定配置文件路径
- `DATABASE_PATH`：覆盖 SQLite 路径
- `SERVER_HOST`：覆盖监听地址
- `SERVER_PORT`：覆盖监听端口

## API 文档

- OpenAPI：`http://127.0.0.1:3001/api/openapi.yaml`
- OpenAPI 源文件：`apps/backend/openapi/openapi.yaml`
- 人类可读文档：`apps/backend/docs/http-api.md`

## 卸载说明

macOS 上完整卸载需要同时删除应用本体和本地数据目录。只删除 `.app` 不会自动清空聊天记录、provider 配置和本地数据库。

推荐步骤：

1. 先从托盘点击 `Quit` 退出应用。
2. 如怀疑未退出干净，可在“活动监视器”里结束 `eidolon-echo-shell` 与 `eidolon-echo-backend`。
3. 删除应用本体，例如 `/Applications/Eidolon-Echo.app`。
4. 删除本地数据目录：
   - `~/Library/Application Support/io.github.hughlfree.eidolonecho`
   - 运行时代码会把后端数据库放到 `app_local_data_dir/backend/chat.db`
5. 如需彻底清理，可再检查并删除：
   - `~/Library/Caches/io.github.hughlfree.eidolonecho`
   - `~/Library/Logs/io.github.hughlfree.eidolonecho`
   - `~/Library/Preferences/io.github.hughlfree.eidolonecho.plist`

终端示例：

```bash
pkill -f eidolon-echo-shell || true
pkill -f eidolon-echo-backend || true

rm -rf "/Applications/Eidolon-Echo.app"
rm -rf "$HOME/Library/Application Support/io.github.hughlfree.eidolonecho"
rm -rf "$HOME/Library/Caches/io.github.hughlfree.eidolonecho"
rm -rf "$HOME/Library/Logs/io.github.hughlfree.eidolonecho"
rm -f "$HOME/Library/Preferences/io.github.hughlfree.eidolonecho.plist"
```

说明：

- 当前 `api_key` 以明文形式保存在本地数据库中，因此删除数据目录才算真正清除本地敏感配置。
- 开发模式默认数据库路径仍是 `apps/backend/data/chat.db`，本地开发时如需彻底清理，也要手动删除该文件。
- 应用内也提供了“清除本地数据”入口：设置中心 -> 其他 -> 本地数据。

## 进程说明

开发模式下，活动监视器中看到以下进程或连接通常是正常现象：

- `eidolon-echo-shell`：桌面壳主进程。
- `eidolon-echo-backend`：本地 Axum 后端 sidecar。
- `127.0.0.1:1420`：Vite 开发服务器连接。多窗口场景下可能出现多条，因为每个 webview 都会连接本地 dev server。
- `Eidolon-Echo ... Graphics and Media`：macOS WebView 渲染相关辅助进程。
- `Eidolon-Echo ... Networking`：macOS WebView 网络相关辅助进程。

正式打包后，前端不再依赖 `127.0.0.1:1420` 的 Vite 开发服务器。

## 关键行为说明（当前）

- `main` 窗口：虚拟形象（拖拽、点按打开菜单）
- `chat` 窗口：输入框（发送消息）
- `bubble` 窗口：回复气泡展示
- `menu` 窗口：菜单和历史
- `settings` 窗口：设置中心（托盘菜单打开）
- 当前会话由后端数据库按 `mode -> profile.active_conversation_id -> conversation` 解析
- 前端/Tauri 不再保存 active conversation id 作为真相源
- 历史消息接口为 `GET /api/messages?mode=...&limit=...&before_id=...`

## CORS 说明

当前后端 CORS 策略分环境：

- `debug`：全开放，便于本地开发和跨端调试。
- `release`：仅允许 `http://tauri.localhost` 与 `tauri://localhost`。
- `release`：仅允许 `GET/POST/PUT/DELETE/OPTIONS` 与 `Accept/Content-Type`。

## 常见问题

- `401 Unauthorized` / `invalid api key`
  - 到设置中心检查当前 mode 绑定的 provider，确认 `api_key`、`base_url`、`model_name` 匹配且 key 有权限。
  - 可先访问 `http://127.0.0.1:3001/api/health`，检查 `ai_precheck.ready` 是否为 `true`。
- `migration ... was previously applied but has been modified` / `... missing in the resolved migrations`
  - 说明本地旧库与当前迁移基线不一致。
  - 开发阶段可删除本地数据库后重启（默认是 `apps/backend/data/chat.db`），让 `0001_init.sql` 重新建库。
