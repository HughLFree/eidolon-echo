# 开发手册（Developer Guide）

本手册面向本项目开发者，聚焦“改功能时该改哪些文件、按什么顺序改”。

## 1. 项目架构速览

- 后端：`apps/backend`
  - `src/main.rs`：后端入口，组装配置、AI client、路由、全局状态。
  - `src/handlers/`：HTTP 层（参数校验、调用 db/ai、返回响应）。
  - `src/db/`：数据访问层（按表/资源拆分）。
  - `src/ai/`：模型调用与上下文缓存管理。
  - `migrations/`：SQLite schema/migration。
  - `openapi/openapi.yaml`：API 契约。
  - `docs/http-api.md`：人类可读接口文档。
- 桌面壳：`apps/desktop/src-tauri`
  - `src/desktop_pet/mod.rs`：托盘、命令注册、启动编排。
  - `src/desktop_pet/commands/`：Tauri 命令入口。
  - `src/desktop_pet/window_manager.rs`：窗口创建/显示/定位/切换。
  - `src/overlay/`：平台差异能力（macOS/windows/fallback）。
- 前端：`apps/desktop/web`
  - `src/*-main.jsx`：多窗口入口（main/chat/bubble/menu/settings）。
  - `src/api/`：对后端的 HTTP 调用封装。
  - `*.html`：每个窗口的挂载页。

## 2. 当前业务基线

- 两种模式：
  - `default`：消息仅内存缓存，不落库消息表。
  - `roleplay`：消息落库，并支持冷启动与下拉分页回填。
- 当前会话解析：
  - 后端按 `mode -> profile.active_conversation_id -> conversation` 解析当前会话。
  - 启动阶段会做一次 bootstrap，确保每个 mode 至少有一个 profile、一个 conversation，且 active 指针已设置。
- 上下文缓存：
  - `ContextManager` 按 `conversation_id` 作为缓存键，避免多会话串上下文。
  - API 拼接上下文优先读缓存，不足再从 DB 回填。

## 3. 本地运行与验证

## 3.1 启动后端

```bash
cargo run -p desktop-ai-backend
```

## 3.2 启动桌面端

```bash
cd apps/desktop/web
npm install
cd ../src-tauri
cargo tauri dev
```

## 3.3 常用检查

```bash
cargo check -p desktop-ai-backend -p desktop-ai-shell
cd apps/desktop/web && npm run build
```

## 4. 常见开发任务

## 4.1 新增后端 API（推荐步骤）

1. 在 `openapi/openapi.yaml` 补路由与 schema。
2. 在 `src/handlers/` 新增或扩展 handler。
3. 在 `src/db/` 新增或扩展数据访问函数。
4. 在 `src/main.rs` 注册路由。
5. 更新 `docs/http-api.md` 示例。
6. 跑 `cargo check`。

## 4.2 修改数据库结构

1. 在 `apps/backend/migrations/` 新增递增编号 SQL（如 `0003_xxx.sql`）。
2. 不直接改线上已有表结构；通过迁移重建/转移数据。
3. 迁移后同步更新 `src/db/` 读写逻辑与类型。
4. 若 API 受影响，同步更新 `openapi.yaml` 与 `http-api.md`。

提示：
- 本项目 migration 由 `sqlx::migrate!` 在启动时自动执行。
- 新增 migration 后，重启后端即可应用。

## 4.3 新增设置项（Provider/Profile）

1. 后端：扩展 `db/providers.rs` 或 `db/profiles.rs` 字段读写。
2. 后端：扩展 `handlers/providers.rs` 或 `handlers/profiles.rs` 请求体校验。
3. 前端：`web/src/api/settings.js` 增加字段传输。
4. 前端：`web/src/settings-main.jsx` 增加表单项与保存逻辑。
5. 文档：更新 `openapi.yaml`、`http-api.md`。

## 4.4 新增桌面窗口

1. 前端：新增 `xxx.html` + `src/xxx-main.jsx` + 样式。
2. Tauri 配置：`src-tauri/tauri.conf.json` 增加窗口定义。
3. 壳层：在 `window_manager.rs` 增加打开/定位逻辑。
4. 命令：必要时在 `commands/` 新增 Tauri command，并在 `desktop_pet/mod.rs` 注册。
5. 托盘或现有窗口触发：补入口动作。

## 5. 代码约定

- 后端分层：
  - `handlers` 不直接写 SQL。
  - SQL 放在 `db/*`，handler 只做组装与校验。
- 前端约定：
  - API 调用统一走 `web/src/api/*`。
  - 窗口入口文件只处理 UI 与事件，不堆积 HTTP 细节。
- 会话上下文：
  - 不在前端/Tauri 维护会话真相源。
  - 会话来源以后端 DB 解析结果为准。

## 6. 排错速查

- 历史消息为空
  - 先看当前 `mode` 是否正确。
  - 检查该 mode 对应 profile 的 `memory_enabled` 是否为 1（roleplay）。
  - 检查 `conversations/messages` 是否有对应数据。
- 设置保存后未生效
  - 检查 `/api/profiles`、`/api/ai-providers` 返回是否更新。
  - 检查请求最终使用的 `provider_id/system_prompt/context_limit`。
- 窗口行为异常（不显示/不定位）
  - 看 `desktop_pet/window_manager.rs` 是否触发了对应 show/sync。
  - 看 `overlay/macos.rs` 日志与面板状态。

## 7. 近期建议（可选）

- 将窗口 label 常量集中管理，降低多处硬编码风险。
- 增加最小回归脚本（后端健康检查 + 前端构建 + 基本路由 smoke test）。
