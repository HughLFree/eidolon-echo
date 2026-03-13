# Eidolon-Echo

Eidolon-Echo 是一个以 AI 对话、角色陪伴为核心的桌面应用。

它不是把 AI 当成“工具栏助手”来设计的。当前更接近一个会在桌面上陪伴你、并能以特定人格和你持续互动的角色。

> [!IMPORTANT]
> **核心目标（尚未完成）**
> 
> 当前项目仍在持续开发中。`1.0` 版本只实现了基础能力，例如本地配置、基础对话、角色模式和历史保存；**还没有完成这个项目真正想做的核心部分: 让 AI 在角色模式下更像一个真实的人，与用户建立更自然、更持续的互动关系。**

---

## 它能做什么

- `default` 模式：轻量对话，不写入消息历史
- `roleplay` 模式：角色模式对话，消息会保存到本地历史
- 点击虚拟形象：弹出迷你菜单，显示“历”按钮
- 历史消息面板：点击“历”后可查看最近消息，支持点开单条详情，再通过 `←` 返回列表
- 气泡回复：回答显示在桌面气泡中，长文本可滚动查看
- 设置中心：统一管理 API、默认模式、角色模式和本地数据清理
- 托盘菜单：切换模式、打开设置、最小化/恢复、退出
- 本地运行：聊天记录和设置默认都保存在用户自己的电脑上

---

## 当前状态

> [!NOTE]
> 当前优先支持 macOS。Windows / Linux 仍是实验状态。

- 当前优先支持 macOS
- Windows / Linux 还没有做完整验证，应视为实验状态
- 项目仍在持续开发中，界面和行为可能继续调整
- 当前版本主要验证了“能用”的基础链路，还没有完成“像真人一样长期陪伴”的核心体验目标

---

## 使用前你需要知道

> [!CAUTION]
> `api_key` 当前以明文形式保存在本地数据库中。

- `roleplay` 模式会把消息保存到本地 SQLite 数据库
- `default` 模式不会把消息写入消息表
- `api_key` 当前以明文形式保存在本地数据库中
- 应用内提供“清除本地数据”功能：设置中心 -> `其他`

如果你对本地隐私比较敏感，这些点需要先接受，再决定是否使用。

---

## 开始使用

> [!TIP]
> 常规流程：启动应用 -> 打开设置中心 -> 填写 API -> 选择模式 -> 开始对话。

如果你拿到的是已经打包好的版本：

1. 安装并启动 `Eidolon-Echo`
2. 打开设置中心
3. 在 `API 设置` 中填写：
   - `base_url`
   - `model_name`
   - `api_key`
4. 选择你要用的模式
5. 开始对话

如果你是开发者，或当前还没有可下载的正式构建，请直接看 [DEVELOPMENT.md](./DEVELOPMENT.md)。

---

## 本地数据

当前应用默认把数据保存在本机。

常见内容包括：

- provider 配置
- profile 配置
- 角色扮演历史消息
- 本地保存的 `api_key`

数据路径取决于你如何运行：

- `cargo tauri dev`（开发模式）：`/Users/<你的用户名>/mycode/desktop-ai/apps/backend/data`
- macOS 打包版（`.app` / `.dmg`）：`~/Library/Application Support/io.github.hughlfree.eidolonecho/backend`

`设置中心 -> 其他 -> 清除本地数据` 清理的是“当前运行模式对应的数据目录”。
也就是：

- 开发模式会清理 `apps/backend/data/chat.db`
- 打包版会清理 `~/Library/Application Support/io.github.hughlfree.eidolonecho/backend/chat.db`

---

## 卸载

只删除 `.app` 不会自动删除本地数据。

如果你想完整卸载：

1. 先从托盘点击 `Quit`
2. 删除应用本体，例如 `/Applications/Eidolon-Echo.app`
3. 删除本地数据目录：
   - `~/Library/Application Support/io.github.hughlfree.eidolonecho`
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

---

## 项目文档

- 开发手册：[DEVELOPMENT.md](./DEVELOPMENT.md)
- 发布检查清单：[RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md)
- 安全说明：[SECURITY.md](./SECURITY.md)
- 后端接口文档：[apps/backend/docs/http-api.md](/Users/hugh/mycode/desktop-ai/apps/backend/docs/http-api.md)

## License

MIT
