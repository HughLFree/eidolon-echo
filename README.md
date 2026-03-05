# desktop-ai

一个以 AI 对话为核心的桌宠项目：
- 前端：Vite + React（由 Tauri 加载）
- 桌面壳：Tauri + Rust
- 后端：Rust（Axum）
- 数据库：SQLite（`sqlx` migration）
- AI：OpenAI-Compatible 协议（默认 DeepSeek）

当前架构是前后端分离：
- `apps/backend` 提供 HTTP API，可独立运行/替换
- `apps/desktop/web` 只依赖 HTTP API
- `apps/desktop/src-tauri` 只负责窗口/桌面能力

## 项目结构

```text
.
├── Cargo.toml
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
│       │   └── src
│       │       ├── desktop_pet/
│       │       │   ├── commands/
│       │       │   │   ├── avatar.rs
│       │       │   │   ├── chat.rs
│       │       │   │   └── menu.rs
│       │       │   ├── mod.rs
│       │       │   ├── state.rs
│       │       │   └── windows.rs
│       │       ├── main.rs
│       │       └── overlay/
│       └── web
│           ├── src/
│           ├── package.json
│           └── vite.config.js
└── README.md
```

## 环境准备

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

## 启动步骤

1. 配置 API Key（环境变量）

```bash
cp apps/backend/.env.example apps/backend/.env.local
export DEEPSEEK_API_KEY="你的 DeepSeek Key"
# 可选
export OPENAI_API_KEY="你的 OpenAI Key"
```

说明：
- 默认配置在 `apps/backend/config/default.toml`
- 默认 provider 是 `deepseek`
- `api_key` 为空时会读取 `api_key_env` 指定的环境变量

2. 启动后端

```bash
cargo run -p desktop-ai-backend
```

后端默认地址：`http://127.0.0.1:3001`

3. 启动桌面端

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

## 关键行为说明

- 主窗口只显示虚拟形象 + 输入框
- 回复在气泡窗口显示
- 点击形象可打开菜单窗口并查看历史
- `session_id` 由前端本地管理（`localStorage`），不再通过 Tauri `invoke` 共享

## CORS 说明

当前后端 CORS 为全开放：
- `allow_origin(Any)`
- `allow_headers(Any)`
- `allow_methods(Any)`

适合本地开发和跨端调试。
生产环境建议改成白名单来源与受限请求方法/请求头。

## 后续扩展建议

- 增加 AI Provider：在 `default.toml` 新增 provider 并切换 `ai.default_provider`
- 替换前端技术栈（如 Swift UI）：保持 HTTP API 协议不变即可
- 增加能力（设置、记忆、总结）：优先扩展 backend handler + migration
