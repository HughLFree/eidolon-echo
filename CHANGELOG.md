# Changelog

## 2026-03-13

### Branding

- 产品名统一改为 `Eidolon-Echo`。
- 应用显示名、窗口标题、OpenAPI 标题、README 标题和卸载路径说明同步更新。
- Rust 包名与活动监视器进程名同步改为 `eidolon-echo-shell` / `eidolon-echo-backend`。

### Backend

- 后端路由与处理器按资源拆分：`handlers` 拆为 `chat/health/providers/profiles/runtime_ai`，入口改为 `handlers/mod.rs` 聚合导出。
- 数据访问层按资源拆分：`db` 拆为 `pool/conversations/messages/providers/profiles`，`db.rs` 保留类型与公共导出。
- 会话主链路统一为 conversation 语义：`/api/messages` 按 mode 解析当前 conversation，不再依赖前端传入会话 ID。
- 上下文缓存改为按 `conversation_id` 建索引，避免同模式多会话时上下文串线。
- `api_key_ref` 统一更名为 `api_key`，明确当前为本地明文存储。
- 启动时增加 AI key 预检查，并通过 `/api/health` 返回 `ai_precheck`。
- `ContextManager` 去掉运行路径中的 `expect` 崩溃点，缓存异常时改为可恢复分支并记录 warning。
- 默认 CORS 改为分环境策略：`debug` 全开放，`release` 仅允许 Tauri 本地来源。

### Database & Migration

- 规范化当前新库初始化 schema：`ai_providers`、`profiles`、`conversations`、`messages` 四张核心表。
- 迁移基线收敛为单文件 `0001_init.sql`，开发阶段通过重建本地库对齐最新 schema。

### Desktop (Tauri)

- `desktop_pet/windows.rs` 重命名为 `window_manager.rs`，并统一调用入口，降低与 `overlay/windows.rs` 的命名混淆。
- 托盘菜单新增设置入口，支持打开设置中心窗口。
- 设置窗口改为标准应用窗口风格，按需创建/显示，减少 hide 逻辑耦合。
- 新增 sidecar 生命周期管理：桌面端启动自动拉起后端，托盘退出和应用退出时尝试关闭 sidecar。
- 新增“清除本地数据”命令：会停后端、删除本地数据目录、重建目录并重启后端。
- 桌面端日志统一切到 `tracing`，减少散落的开发态 `eprintln!`。

### Web (React)

- 新增设置中心页面与样式：`settings.html` + `settings-main.jsx` + `settings.css`。
- 新增设置 API 封装：`src/api/settings.js`，覆盖 provider/profile 读取与保存。
- 设置中心整理为五页：`概览 / API 设置 / 默认模式 / 角色扮演 / 其他`。
- “清除本地数据”入口移动到 `其他` 页面，避免和普通配置项混放。
- 历史消息请求链路改为 `GET /api/messages` + mode 查询参数。
- 菜单历史加载链路修复：最新在上、分页下拉、缓存优先、DB 回填。

### Testing

- 增加最小 smoke tests，覆盖：
  - 首次启动建库
  - 已有数据库重启
  - 无 key 启动
  - 错误 key 提示
  - 默认模式发送消息
  - roleplay 模式历史加载
  - sidecar 终止与数据目录重建

### Docs

- README、开发手册、API 文档同步到最新命名、启动方式、设置页结构和卸载/清理流程。

## 2026-03-06

- 重构桌宠窗口模型：由旧 `dialog` 方案切换为 `main/chat/bubble/menu` 四窗口。
- 完成 macOS overlay 能力接入：`NSPanel`、`CanJoinAllSpaces`、`FullScreenAuxiliary`、`Stationary`。
- 调整 overlay 平台分发：`macos/windows/fallback` 按 `cfg` 路径隔离实现。
- 菜单窗口默认改为启动隐藏，仅在点击 avatar 后显示；并补齐菜单 keepalive/历史面板交互链路。
- 前端调用链清理：删除 `dialog` 入口与样式，拆分 `chat.html + chat-main.jsx + chat.css`，移除未使用代码。
- README 更新为当前前后端分离结构与运行说明。
- 补齐 Windows 构建所需图标文件：`apps/desktop/src-tauri/icons/icon.ico`。
