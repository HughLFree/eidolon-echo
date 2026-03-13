# Changelog

## 2026-03-13

### Backend

- 后端路由与处理器按资源拆分：`handlers` 拆为 `chat/health/providers/profiles/runtime_ai`，入口改为 `handlers/mod.rs` 聚合导出。
- 数据访问层按资源拆分：`db` 拆为 `pool/conversations/messages/providers/profiles`，`db.rs` 保留类型与公共导出。
- 会话主链路统一为 conversation 语义：`/api/messages` 按 mode 解析当前 conversation，不再依赖前端传入会话 ID。
- 上下文缓存改为按 `conversation_id` 建索引，避免同模式多会话时上下文串线。
- 引入 provider/profile CRUD，并将运行时 AI 参数解析与校验串到请求链路。
- profile 配置生效链路打通：系统提示词、context_limit、memory_enabled、provider 选择可从数据库读取并参与请求。

### Database & Migration

- 规范化新库初始化 schema：`ai_providers`、`profiles`、`conversations`、`messages` 四张核心表。
- 新增 migration `0002_conversation_ids_to_integer.sql`，将 `conversations.id` 与 `messages.conversation_id` 统一迁移为 `INTEGER`。
- 消息读写与查询绑定统一为 `i64`，移除字符串转换与 `parse` 回退路径。
- conversation 生成与解析逻辑改造为纯数值 ID 流程，减少类型歧义。

### Desktop (Tauri)

- `desktop_pet/windows.rs` 重命名为 `window_manager.rs`，并统一调用入口，降低与 `overlay/windows.rs` 的命名混淆。
- 托盘菜单新增设置入口，支持打开设置中心窗口。
- 设置窗口改为标准应用窗口风格，按需创建/显示，减少 hide 逻辑耦合。
- 清理前端/Tauri 侧 active session/conversation 兜底状态，避免多源状态分叉。

### Web (React)

- 新增设置中心页面与样式：`settings.html` + `settings-main.jsx` + `settings.css`。
- 新增设置 API 封装：`src/api/settings.js`，覆盖 provider/profile 读取与保存。
- 历史消息请求链路改为 `GET /api/messages` + mode 查询参数。
- 移除旧 `session.js` 与相关 localStorage 恢复逻辑。
- 菜单历史加载链路修复：最新在上、分页下拉、缓存优先、DB 回填。

### Config & Prompt

- 环境文件改为 `.env.default` 主路径，清理 `.env.example`。
- 默认 prompt 文件统一命名为 `*.default.md`，并同步读取路径。
- 补充 `max_tokens` 等 provider 配置项，并统一配置加载校验逻辑。

### Docs

- OpenAPI 与人类可读 API 文档更新到最新路由与数据结构。
- README 全量更新：目录结构、当前功能、配置项、会话链路说明。
- 新增 `DEVELOPMENT.md` 开发手册（架构、开发流程、排错与扩展建议）。

## 2026-03-06

- 重构桌宠窗口模型：由旧 `dialog` 方案切换为 `main/chat/bubble/menu` 四窗口。
- 完成 macOS overlay 能力接入：`NSPanel`、`CanJoinAllSpaces`、`FullScreenAuxiliary`、`Stationary`。
- 调整 overlay 平台分发：`macos/windows/fallback` 按 `cfg` 路径隔离实现。
- 菜单窗口默认改为启动隐藏，仅在点击 avatar 后显示；并补齐菜单 keepalive/历史面板交互链路。
- 前端调用链清理：删除 `dialog` 入口与样式，拆分 `chat.html + chat-main.jsx + chat.css`，移除未使用代码。
- README 更新为当前前后端分离结构与运行说明。
- 补齐 Windows 构建所需图标文件：`apps/desktop/src-tauri/icons/icon.ico`。
