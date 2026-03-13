# desktop-ai

一个以 AI 对话为核心的桌宠项目。

- 前端：Vite + React（由 Tauri 加载）
- 桌面壳：Tauri + Rust
- 后端：Rust（Axum）
- 数据库：SQLite（`sqlx` migration）
- AI 协议：OpenAI-Compatible（默认 DeepSeek）

当前是前后端分离架构：

- `apps/backend` 提供 HTTP API，可独立运行/替换
- `apps/desktop/web` 只依赖 HTTP API
- `apps/desktop/src-tauri` 只负责窗口管理与桌面能力

注意：`ref/` 是参考代码目录，已加入 `.gitignore`，不属于主工程结构。

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

评估时间：当前版本（忽略 `ref/`）

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
cd apps/desktop/web
npm install
cd ../src-tauri
cargo tauri dev
```

说明：

- `cargo tauri dev` 会自动拉起后端 sidecar（默认 `127.0.0.1:3001`）。
- 托盘 `Quit` 或应用退出事件会尝试结束 sidecar 后端进程。
- 开发模式下后端数据库路径为 `apps/backend/data/chat.db`。

### 3) 配置 AI Provider（推荐在设置中心）

请在设置中心的 API 页填写 provider 字段（会写入数据库）：

- `base_url`
- `model_name`
- `api_key_ref`（这里直接填真实 API key 字符串）
- `temperature`、`max_tokens`（可选）

说明：

- 系统会在后端启动时做一次 key 预检查，并在 `GET /api/health` 的 `ai_precheck` 返回结果。
- 默认配置文件是 `apps/backend/config/default.toml`，主要用于初始值与无数据库配置时兜底。
- 数据库迁移会在启动时自动执行（`sqlx` migrator）。
- `max_tokens` 建议填正整数；留空表示不限制。

### 4) 独立启动后端（可选）

仅在你需要单独调试后端时使用：

```bash
cargo run -p desktop-ai-backend
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

当前后端 CORS 为全开放：

- `allow_origin(Any)`
- `allow_headers(Any)`
- `allow_methods(Any)`

适合本地开发和跨端调试。生产环境建议改成白名单来源与受限方法/请求头。

## 常见问题

- `401 Unauthorized` / `invalid api key`
  - 到设置中心检查当前 mode 绑定的 provider，确认 `api_key_ref`、`base_url`、`model_name` 匹配且 key 有权限。
  - 可先访问 `http://127.0.0.1:3001/api/health`，检查 `ai_precheck.ready` 是否为 `true`。
- `migration ... was previously applied but has been modified` / `... missing in the resolved migrations`
  - 说明本地旧库与当前迁移基线不一致。
  - 开发阶段可删除本地数据库后重启（默认是 `apps/backend/data/chat.db`），让 `0001_init.sql` 重新建库。
