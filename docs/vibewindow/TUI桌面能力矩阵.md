# TUI 与桌面能力矩阵

## 判定口径

本矩阵以 2026-08-24 的活动调用链为准。`活动可达` 必须能从 `V3Action` 或命令目录到达 TUI handler，再到真实 Gateway 调用或本地展示动作；`部分接入` 只承认已有闭环，不把同领域的未接能力一并升级；仅有 Gateway client、休眠模型或旧 widget 的能力归为 `完全未接入`。

## 活动可达

| 能力 | 活动证据 | 安全/失败边界 |
|---|---|---|
| chat stream 与 transcript | `SubmitPrompt` → resumable chat stream → bridge/reducer/ChatWidget | 单一终态；断线、超时、错误可见 |
| 思考、工具、步骤与结构化内容 | typed stream adapter → `UiMessage` → V3 events | 未知事件显式诊断；不展示被脱敏思考 |
| 权限、提问与 Workflow pause | pending list/pause → overlay → permission/question reply 或 workflow resume | 请求、会话、run 与 pause token 归属均校验；token 不进入 UI 状态 |
| 会话恢复、新建与变更 | session load/create/rename/fork/reset/summarize/archive API | 持久化锚点、忙碌门禁、类型确认；失败前不切换本地状态 |
| 模型与会话选项 | provider/config/acp 能力 → `SessionOptions` | 只接受服务端声明能力；危险升级确认；不写全局配置 |
| 工作目录切换 | `/cd` → `project_resolve(false)` → 临时 runtime 完整 bootstrap → 原子提交 | 不创建未知项目；活动流、排队输入、并发切换或 memory mutation 时拒绝 |
| Gateway 诊断 | `/doctor` → `doctor_get` → 受限诊断摘要 | 丢弃迟到目录结果；错误、类别和消息脱敏限长 |
| Hook 安全管理 | `/hooks` → catalog/audit；当前快照 + 确认 → trust/revoke → refresh | stable plugin/hash、当前目录快照、单 mutation 与响应一致性校验 |
| Memory 查询与 core 写删 | `/memory` → query；确认后 store/delete → 按原 query 刷新 | 删除只接受当前目录查询快照精确 key；切目录与并发 mutation 门禁 |
| 计划与持续目标 | `/plan <request>`、`/goal <objective>` → task-plan-run SSE | request/task 双 ID 归属；批次、子任务、等待、阻塞与终态可见 |

## 部分接入

| 能力 | 已接入部分 | 尚未接入/明确边界 |
|---|---|---|
| 工作区文件 | `@` 文件名搜索、只读 `/files` 浏览与文本读取、附件提交、流内结构化 diff、`/diff` 会话差异 | 无内容搜索、Git 状态面板及 file write/move/copy/delete；聊天工具经审批改文件不等于文件管理命令 |
| Skill 与 Plugin | 目录、stable ID prompt、启停、删除 | 无创建、安装、市场、升级或显式 reload 命令 |
| MCP | `/mcp` 通过 `mcp_inspect` 展示服务摘要 | 无 prompt/resource/tool 执行；不注册 `/mcp__...` |
| Workflow | 列表、详情、`/workflow:<uuid>` 流式运行、pause 表单与精确恢复 | 无创建、编辑或删除 Workflow 的 TUI 管理面 |
| Cron | jobs 列表、job/runs 详情、owned ID 启停与删除 | 无创建和任意字段编辑；投递、模型与 full-access 等敏感参数不开放 |
| Scheduler | workers/tasks 目录、owned task graph/受限日志/产物元数据详情，以及按 canonical 状态门禁的 start/retry/pause/resume/cancel/archive | 无任务创建、指派、任意状态写入、任务图替换或 team/staff/worker 管理 |
| Project/worktree/Git | `/cd` 只用 project resolve 切换 Gateway 已知目录 | 无 Project CRUD、worktree 创建/删除/reset、commit/merge 或任意 Git command |
| 会话 todo | Gateway 事件可只读显示 todo | 无人工新增、编辑或删除入口 |

## 完全未接入（本轮范围外）

| 能力 | 分类 | 当前边界 |
|---|---|---|
| 知识库 | 完全未接入 | dataset、document、index、embedding 与 retrieve 均无活动 TUI 路由；需要独立产品面 |
| PTY | 完全未接入 | terminal create/resize/WebSocket I/O/delete 均无活动 TUI 路由；不能用本地命令执行伪装 |
| APIQL | 完全未接入 | Gateway client 存在不等于活动能力；未设计查询目录、结果边界与授权面 |
| Provider 认证 | 完全未接入 | OAuth、API key 写入/移除及 credential 管理无活动路由；模型选择不等于认证管理 |
| 全局/租户配置 | 完全未接入 | `/settings` 只改会话选项，不读取或 patch 全局配置 |
| Shiju 账号、应用与技能市场 | 完全未接入 | 登录、credential、market browse/install/upgrade 均无活动路由；`/skills`/`/plugins` 仅管理已加载目录 |
| 额外目录授权、剪贴板写入、语音与快捷键持久化 | 完全未接入 | 缺少统一跨平台 Gateway 契约，不启用旧 TUI 休眠面板 |

## 桌面或运维专属

| 能力 | 分类 | 不进入 TUI 的原因 |
|---|---|---|
| 窗口、托盘、系统菜单与多窗口导航 | 桌面专属 | 依赖 GUI 窗口宿主 |
| WebView、浏览器会话与网页授权 | 桌面专属 | 依赖桌面 WebView 与登录态 |
| 系统通知、Dock、窗口注意力与拖放/文件选择器 | 桌面专属 | 依赖操作系统 GUI 生命周期；TUI 只接受明确附件路径或粘贴内容 |
| 图片预览、GUI 主题、布局、动画与可视化面板 | 桌面专属 | TUI 只维护终端主题、文本布局和结构化文本展示 |
| 自动更新、安装器、应用签名与外部应用启动 | 桌面专属 | 属于桌面分发、签名与应用宿主链路 |
| Gateway 实例释放、服务/租户运维与 credential 写入 | 运维专属 | 会扩大管理权限或暴露敏感信息，不纳入聊天 TUI 核心命令 |
| Staff/team/worker 管理与内部调度事件 | 运维专属 | 当前仅展示 scoped workers/tasks；组织级控制需要独立授权和管理界面 |

## 静态调用链审计

```text
V3Action / slash command
  -> GatewayV3Bridge / CommandRoute
  -> ActiveRunner / V3GatewayApp handler
  -> GatewayUiRuntime / GatewayClient
  -> UiRuntimeEvent reducer or bounded host result
  -> UiMessage + V3DisplayEvent
  -> ChatWidget transcript / overlay
```

- `TaskStateChanged` 只触发当前会话 interaction snapshot 刷新；权限与提问回复携带原始 request ID。
- 目录、查询、会话和 mutation 均使用 generation/owned snapshot 校验，迟到结果不得覆盖新状态。
- `@` 搜索与 `/files` 都限制在当前 Gateway 目录；前者只搜文件名，后者只读目录和文本文件。
- `UsageUpdated` 被显式识别，不会被误报为未知事件；实际显示只使用 Gateway 返回的 usage。
- 桌面/运维专属项不在 `COMMANDS` 注册表，也不通过动态 Skill 自动获得本地系统或管理权限。
