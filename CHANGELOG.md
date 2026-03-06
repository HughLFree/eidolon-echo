# Changelog

## 2026-03-06

- 重构桌宠窗口模型：由旧 `dialog` 方案切换为 `main/chat/bubble/menu` 四窗口。
- 完成 macOS overlay 能力接入：`NSPanel`、`CanJoinAllSpaces`、`FullScreenAuxiliary`、`Stationary`。
- 调整 overlay 平台分发：`macos/windows/fallback` 按 `cfg` 路径隔离实现。
- 菜单窗口默认改为启动隐藏，仅在点击 avatar 后显示；并补齐菜单 keepalive/历史面板交互链路。
- 前端调用链清理：删除 `dialog` 入口与样式，拆分 `chat.html + chat-main.jsx + chat.css`，移除未使用代码。
- README 更新为当前前后端分离结构与运行说明。
- 补齐 Windows 构建所需图标文件：`apps/desktop/src-tauri/icons/icon.ico`。
