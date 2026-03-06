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
│   │       ├── db.rs
│   │       ├── handlers.rs
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
│       │       │   │   └── overlay.rs
│       │       │   ├── state.rs
│       │       │   ├── windows.rs
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
│           ├── src/
│           └── vite.config.js
└── README.md
```

## 结构评估

评估时间：当前版本（忽略 `ref/`）

### 优点

- 分层清晰：`backend` / `web` / `src-tauri` 职责边界明确。
- 桌面域模型可读性较好：`desktop_pet` 里按 `commands + state + windows` 组织。
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
4. 增加 `README` 的“故障排查”章节，明确日志关键词与定位方式。

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

### 2) 配置 API Key

```bash
cp apps/backend/.env.example apps/backend/.env.local
export DEEPSEEK_API_KEY="你的 DeepSeek Key"
# 可选
export OPENAI_API_KEY="你的 OpenAI Key"
```

说明：

- 默认配置在 `apps/backend/config/default.toml`
- 默认 provider 是 `deepseek`
- `api_key` 为空时会读取 `api_key_env` 指定环境变量

### 3) 启动后端

```bash
cargo run -p desktop-ai-backend
```

后端默认地址：`http://127.0.0.1:3001`

### 4) 启动桌面端

```bash
cd apps/desktop/web
npm install
cd ../src-tauri
cargo tauri dev
```

前端默认请求：`http://127.0.0.1:3001`

如需修改前端请求地址：

```bash
export VITE_BACKEND_BASE_URL=http://127.0.0.1:3001
```

或创建 `apps/desktop/web/.env.local`：

```bash
VITE_BACKEND_BASE_URL=http://127.0.0.1:3001
```

## API 文档

- OpenAPI：`http://127.0.0.1:3001/api/openapi.yaml`
- OpenAPI 源文件：`apps/backend/openapi/openapi.yaml`
- 人类可读文档：`apps/backend/docs/http-api.md`

## 关键行为说明（当前）

- `main` 窗口：虚拟形象（拖拽、点按打开菜单）
- `chat` 窗口：输入框（发送消息）
- `bubble` 窗口：回复气泡展示
- `menu` 窗口：菜单和历史
- `session_id` 由前端本地管理（`localStorage`）

## CORS 说明

当前后端 CORS 为全开放：

- `allow_origin(Any)`
- `allow_headers(Any)`
- `allow_methods(Any)`

适合本地开发和跨端调试。生产环境建议改成白名单来源与受限方法/请求头。
