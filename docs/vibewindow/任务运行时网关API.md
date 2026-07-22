# 任务运行时网关 API

任务运行时网关 API 是 JS 桌面客户端、Rust 桌面兼容层与 Agent Scheduler
交互的统一边界。任务看板只读取 gateway 返回的快照和事件，并通过 command
提交用户意图；状态推进、调度候选、审核结果、合并结果和 worktree 资源分配都由
gateway/Agent Scheduler 计算。

## 租户请求契约

所有客户端业务 API 都以请求 header 作为唯一租户来源，URL 不再携带平台或公司路径段：

```http
X-Platform-ID: 10000
X-Company-ID: 100000000
```

- 缺少显式配置时，Desktop 网关配置使用默认平台 `10000` 和默认公司 `100000000`。
- Desktop 选择网关或修改该网关的 platform/company 后，新创建的普通、阻塞和流式请求立即使用当前网关保存的租户。
- 非 `OPTIONS` 请求缺少、重复、非 UTF-8 或仅含空白的任一租户 header 时返回 `400 Bad Request`，服务端不会回退到默认租户。
- `OPTIONS` 预检不要求租户 header。第三方 webhook（`/v1/webhook`、WhatsApp、Linq、WATI、Nextcloud Talk、QQ）无法控制协议 header，因此豁免。
- 根 `/health` 是外部探针并豁免；客户端 `/v1/health` 仍属于受管路由，必须携带两个租户 header。

`project_id` 等资源 ID 只标识租户内资源。相同资源 ID 可存在于不同租户，handler 从请求 extension 获取租户并传入 Store，不能从路径、请求体或默认值重建租户。

## 首屏加载顺序

JS 客户端进入项目任务看板时使用以下最小序列：

1. `POST /v1/projects/resolve` 解析 `project_id`。
2. `GET /v1/task-lifecycle/workflows` 拉取可选 workflow 定义。
3. `GET /v1/agent-scheduler/projects/{project_id}/workflow` 拉取项目 workflow 投影。
4. `GET /v1/staff/projects/{project_id}/projection` 拉取员工、阶段负责人和动作投影。
5. `GET /v1/agent-scheduler/projects/{project_id}/tasks` 拉取任务板快照。
6. `POST /v1/task-pool/worktrees/snapshot` 拉取 worktree resource 状态。
7. 对首屏可见或详情打开的任务调用 `GET /v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/logs`。
8. 记录快照返回或事件流返回的最大 cursor，随后连接 SSE。

首屏 UI 必须以 `AgentSchedulerTaskBoardStatusResponse.tasks` 和
`read_model.workflow` 渲染列、卡片、负责人、阶段动作和 workspace 标记。客户端可以保留内存
cache，但 cache 只能用于渲染和乐观禁用按钮，不能作为状态推进依据。

## Lifecycle 推进边界

自动状态推进由 gateway/Agent Scheduler 统一处理。自动路径只提交 command、worker event 或阶段能力 outcome，不提交
目标 `status_key`：

- `run_stage_action` 结果中的 success、failure、blocked 由当前 lifecycle transition 解析为下一阶段。
- dispatch 只选择 lifecycle role 可调度且运行态投影为 queued 的任务。
- policy 和 worker event 写状态时必须同步 `status_key`、`status_role` 和兼容 `status`。
- 缺少 workflow、transition、启用状态或 role 运行态映射时返回明确错误或拒绝，不使用 `move` 补偿。

`AgentTaskStatus` 是运行态兼容投影，不能覆盖 lifecycle `status_key`。客户端读列和阶段时以 gateway 快照中的
`status_key` / `read_model.status_role` 为准。

## 事件订阅

事件回放使用：

```http
GET /v1/agent-scheduler/projects/{project_id}/events?after_cursor={cursor}
```

实时订阅使用：

```http
GET /v1/agent-scheduler/projects/{project_id}/events/stream?after_cursor={cursor}
Accept: text/event-stream
```

SSE 每帧的 `data:` 是 `AgentSchedulerTaskBoardEventDto` JSON。客户端处理规则：

- cursor 单调前进，已处理 cursor 需要幂等跳过。
- `TaskCreated`、`HumanIntervened`、`TaskArchived`、`TaskPaused`、`TaskCompleted` 等事件只用于局部刷新提示。
- 事件 payload 不足以重建完整卡片时，刷新 `GET /tasks`。
- 任务详情打开时，收到相关事件后刷新该任务日志和产物。
- SSE 断开后用最后 cursor 先调用 `/events` 补齐，再重新连接 `/events/stream`。

## Command 写入口

任务看板用户意图统一提交 command：

```http
POST /v1/agent-scheduler/projects/{project_id}/commands
POST /v1/agent-scheduler/projects/{project_id}/commands/bulk
```

最小请求字段：

```json
{
  "task_id": "task-123",
  "command": "move",
  "dispatch_mode": "local",
  "reason": "手动移动任务",
  "payload": {
    "move_task": {
      "from_status_key": "pool",
      "to_status_key": "pending",
      "to_status": "queued",
      "order_no": 3
    }
  }
}
```

命令语义：

| command | 客户端触发场景 | payload |
| --- | --- | --- |
| `move` | 拖拽、手动批量移动；临时人工修正 | `move_task` |
| `update_metadata` | 编辑标题、prompt、模型、负责人、优先级、阶段字段 | `metadata_update` |
| `start` / `retry` / `pause` / `resume` / `cancel` | 执行控制 | 可为空 |
| `archive` / `soft_delete` / `restore` | 移除、归档、恢复 | 可为空 |
| `promote` | 从 backlog/池进入可执行队列 | 可为空 |
| `merge` | 进入合并动作 | 可为空 |
| `assign_staff` | 修改阶段员工 | `staff_command` |
| `run_stage_action` | 运行阶段动作或 saved workflow | `stage_action` |
| `import_legacy_task` | 兼容导入旧桌面任务 | `legacy_import` |

`AgentSchedulerCommandResponse.accepted=false` 表示 gateway 拒绝该意图；客户端显示错误并刷新任务快照。
批量命令设置 `stop_on_error=true` 时，任一命令失败后停止后续写入。

`Move` 仍保留为人工纠错入口，可以用于拖拽和批量移动，但自动调度不得依赖它。stage action、dispatch、policy 或
worker event 需要改变阶段时，必须走 lifecycle outcome 或对应受控命令；如果 transition 不存在，应拒绝并报告原因。

## 任务创建与导入

新任务使用：

```http
POST /v1/agent-scheduler/tasks
```

`AgentSchedulerCreateTaskRequest` 至少包含 `project_path`、`task_id`、`title`、`kind`、
`spec_uri`、`meta_uri`、`created_at_ms` 和任务板 metadata。`metadata.task_board.*`
字段承载 `workflow_id`、`status_key`、`priority`、`prompt`、`model`、`acp_agent`、
`stage_id`、`staff`、`stage_action` 等读模型来源。

旧桌面任务迁移只允许通过：

```http
POST /v1/agent-scheduler/projects/{project_id}/legacy-import
```

迁移完成后客户端继续读取 `/tasks`，不要继续从旧文件推导任务列。

## 日志与产物刷新

日志读取：

```http
GET /v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/logs
```

产物读取：

```http
GET /v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/artifacts
```

日志是审计与展示流，不是状态机输入。客户端不能根据日志文本推断成功、失败、审核通过或合并完成；
这些结论必须来自 `/tasks` 快照或 gateway event。产物列表只展示 gateway 返回的 `kind`、`name`、
`path` 和 `payload`，不要扫描 worktree 路径补齐。

## Workflow、Staff 与 Worktree

Workflow 定义由 `GET /v1/task-lifecycle/workflows` 和
`GET /v1/agent-scheduler/projects/{project_id}/workflow` 提供。任务卡片优先使用
`AgentSchedulerTaskBoardTaskReadModelDto.workflow_id`、`status_label`、`status_role`、
`stage_id`、`stage_label`、`stage_action` 与 `available_stage_actions`。

Staff 投影由 `GET /v1/staff/projects/{project_id}/projection` 提供；staff 变更通过
`POST /v1/staff/projects/{project_id}/commands` 或 agent scheduler 的 `assign_staff`
command 写入。

Worktree resource API 只管理 workspace、slot lease、回收和 merge lock：

- `POST /v1/task-pool/worktrees/snapshot`
- `POST /v1/task-pool/worktrees/acquire`
- `POST /v1/task-pool/worktrees/release`
- `POST /v1/task-pool/worktrees/recycle`
- `POST /v1/task-pool/worktrees/slots/recycle`
- `POST /v1/task-pool/worktrees/merge-lock`
- `POST /v1/task-pool/worktrees/merge-unlock`

客户端不得用 worktree 池结果挑选下一任务，也不得用本地 git 状态覆盖 gateway 的 review/merge 结果。

## 安全边界

- 每个受管请求必须同时发送 `X-Platform-ID` 与 `X-Company-ID`；不要在 URL、query 或 body 中复制租户真相来源。
- 不在日志、metadata 或事件 payload 中写入密钥、原始令牌、完整 prompt 以外的敏感载荷。
- command 只表达意图和最小上下文；执行权限由 gateway 策略判断。
- 客户端重试必须保持幂等，重复 command 需要可由 gateway 拒绝或合并。
- UI cache 清空、重连或跨版本升级后，必须从 `/tasks` 和 `/workflow` 重新建立视图。
