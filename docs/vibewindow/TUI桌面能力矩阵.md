# TUI 与桌面能力矩阵

## 判定口径

本矩阵以 2026-08-24 的活动调用链为准。`活动可达` 必须能从 `V3Action` 或命令目录到达 TUI handler，再到真实 Gateway 调用或本地展示动作；`明确不可用` 不因存在休眠模型而升级；`桌面专属` 不进入 TUI 命令目录。

## 活动可达（12 类）

| 能力 | 分类 | 活动证据 | 安全/失败边界 |
|---|---|---|---|
| chat stream 与 transcript | 活动可达 | `SubmitPrompt` → resumable chat stream → bridge/reducer/ChatWidget | 单一终态；断线、超时、错误可见 |
| 思考、工具、步骤与结构化内容 | 活动可达 | typed stream adapter → `UiMessage` → V3 events | 未知事件显式诊断；不展示被脱敏思考 |
| 权限与提问 | 活动可达 | pending list → overlay → `permission_reply`/`question_reply` | 请求 ID 与当前会话匹配 |
| 会话恢复与新建 | 活动可达 | `session_resume_candidates`、`session_ui_load`、`session_create` | 失败前不切换本地状态 |
| 会话变更 | 活动可达 | rename/fork/reset/summarize/archive Gateway API | 持久化锚点、忙碌门禁、类型确认 |
| 模型与会话选项 | 活动可达 | `provider_list`/config/acp 能力 → `SessionOptions` | 只接受服务端声明能力；危险升级确认 |
| 文件、图片、粘贴与资源附件 | 活动可达 | workspace canonicalization → structured content blocks | file-only、边界、大小、MIME、UTF-8 校验 |
| Skill 与 Plugin | 活动可达 | 并发目录加载、stable ID prompt/toggle/delete | 来源隔离、确定性去重、删除确认 |
| MCP 与记忆 | 活动可达 | `mcp_inspect`、`memory_query_get` 只读面板 | 不伪造执行或写入能力 |
| Workflow、Cron、Agent 与任务 | 活动可达 | 四源并发 canonical snapshot → 只读面板 | 单来源失败隔离；不推断状态 |
| 当前项目任务取消 | 活动可达 | snapshot owned ID + canonical 状态 + scope → scheduler command | 非当前项目或不可取消状态拒绝 |
| 计划与持续目标 | 活动可达 | `/plan <request>`、`/goal <objective>` → task-plan-run SSE | request/task 双 ID 归属；批次、子任务、等待、阻塞与终态可见 |

## 明确不可用（6 类）

| 能力 | 分类 | 当前边界 |
|---|---|---|
| Workflow 执行/恢复命令 | 明确不可用（后端阻塞） | Gateway client 能力未接入最终 TUI 命令目录 |
| MCP prompt 执行 | 明确不可用（后端阻塞） | MCP 目录只读，无 `/mcp__...` 活动路由 |
| Cron 启停、编辑、删除与历史 | 明确不可用（后端阻塞） | `/schedule` 只展示 canonical 列表 |
| Agent 创建、指派与任意任务控制 | 明确不可用（后端阻塞） | 仅 worker 只读与当前项目任务 cancel |
| 额外目录授权、剪贴板、语音、快捷键写入 | 明确不可用（后端阻塞） | 缺少跨平台授权与持久化契约 |
| 旧 TUI 休眠面板/检查命令 | 明确不可用（后端阻塞） | 未进入 `commands.rs` 与 `app.rs` 活动调用链 |

## 桌面专属（6 类）

| 能力 | 分类 | 不进入 TUI 的原因 |
|---|---|---|
| 窗口、托盘、系统菜单与多窗口导航 | 桌面专属 | 依赖 GUI 窗口宿主 |
| WebView、浏览器会话与网页授权 | 桌面专属 | 依赖桌面 WebView 与登录态 |
| 系统通知、Dock 与窗口注意力 | 桌面专属 | 依赖操作系统 GUI 生命周期 |
| 拖放、系统文件选择器与图片预览 | 桌面专属 | TUI 仅接受明确附件路径/粘贴内容 |
| 自动更新、安装器与应用签名 | 桌面专属 | 属于桌面分发和签名链路 |
| GUI 主题、布局、动画与可视化面板 | 桌面专属 | TUI 只维护终端主题与文本布局 |

## 静态调用链审计

```text
V3Action / slash command
  -> GatewayV3Bridge / CommandRoute
  -> ActiveRunner / V3GatewayApp handler
  -> GatewayUiRuntime / GatewayClient
  -> UiRuntimeEvent reducer
  -> UiMessage + V3Event
  -> ChatWidget transcript / overlay
```

- `TaskStateChanged` 只触发当前会话 interaction snapshot 刷新；权限与提问回复均携带原始 request ID。
- `UsageUpdated` 被显式识别，不会被误报为未知事件；实际显示只使用 Gateway 返回的 usage。
- plan-run 通过 `adapt_task_plan_run_event` 进入同一 reducer，终态清理 owned request/task ID。
- 桌面专属项不在 `COMMANDS` 注册表，也不通过动态 Skill 自动获得本地系统权限。
