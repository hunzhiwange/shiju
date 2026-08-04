# 任务工作流配置示例

1234
文给出开发生命周期、营销活动执行和招聘流程的最小配置示例。示例字段展示调度语义，
不是完整用户配置模板。2

生命周期定义通过 `vw-agent` 的 `/v1/task-lifecycle/workflows` API 保存。桌面端 Workflow Manager、
任务看板和下一版 JS 客户端只消费 gateway 返回的 DTO/cache；下面的 TOML 片段是项目选择 workflow、
阶段负责人和阶段能力的配置，不是生命周期模板的本地保存格式。

## 生命周期 API DTO 示例
本文给出开发生命周期、营销活动执行和招聘流程的最小配置示例。示例字段展示调度语义，
不是完整用户配置模板。2

生命周期定义通过 `vw-agent` 的 `/v1/task-lifecycle/workflows` API 保存。桌面端 Workflow Manager、
任务看板和下一版 JS 客户端只消费 gateway 返回的 DTO/cache；下面的 TOML 片段是项目选择 workflow、
阶段负责人和阶段能力的一下配置，不是生命周期模板的本地保存格式。

## 生命周期 API DTO 示例

创建或更新自定义生命周期时，请向 `POST /v1/task-lifecycle/workflows` 或
`PUT /v1/task-lifecycle/workflows/{id}` 提交 `TaskLifecycleWorkflowUpsertBody`：

```json
{
  "workflow": {
    "id": "support_review",
    "label": "Support Review",
    "statuses": [
      {
        "key": "support_queued",
        "label": "待分派",
        "role": "queued",
        "auto_execution_candidate": true,
        "is_scheduling_backlog": true,
        "next": "support_working"
      },
      {
        "key": "support_working",
        "label": "处理中",
        "role": "executing",
        "auto_execution_candidate": true,
        "occupies_execution_slot": true,
        "next": "support_review_ready",
        "transitions": [
          {
            "outcome": "failed",
            "label": "退回复查",
            "target": "support_queued"
          }
        ]
      },
      {
        "key": "support_review_ready",
        "label": "待复查",
        "role": "review_ready",
        "next": "support_done"
      },
      {
        "key": "support_done",
        "label": "已完成",
        "role": "done",
        "terminal": true
      }
    ]
  }
}
```

`GET /v1/task-lifecycle/workflows` 会返回内置和自定义生命周期。内置 workflow 带
`"built_in": true`，不能通过 create/update/delete 覆盖。旧 `task-workflows.json` 只由
`vw-agent` 在生命周期 SQLite 为空时迁移一次；桌面端不再保存这个文件。

## Gateway 任务板读模型片段

JS 客户端首屏读取 `GET /v1/agent-scheduler/projects/{project_id}/tasks`。以下片段展示
workflow/status role/stage action/staff 字段在任务卡片上的最小形态：

```json
{
  "project_id": "proj_123",
  "read_model": {
    "workflow": {
      "id": "dev_lifecycle",
      "label": "Development Lifecycle",
      "statuses": [
        {
          "key": "pending",
          "label": "待执行",
          "role": "queued",
          "auto_execution_candidate": true,
          "next": "running"
        },
        {
          "key": "code_review",
          "label": "代码审核",
          "role": "review",
          "next": "pr_submitted"
        }
      ]
    }
  },
  "tasks": [
    {
      "project_id": "proj_123",
      "task_id": "task_001",
      "status_key": "code_review",
      "status": "submitted",
      "priority": 5,
      "order_no": 10,
      "model": "auto",
      "prompt": "修复登录重试提示",
      "read_model": {
        "title": "修复登录重试提示",
        "workflow_id": "dev_lifecycle",
        "status_label": "代码审核",
        "status_role": "review",
        "stage_id": "code_review",
        "stage_label": "代码审核",
        "staff": {
          "member_id": "staff-reviewer",
          "member_role": "reviewer",
          "primary_member_id": "staff-reviewer",
          "backup_member_ids": ["staff-reviewer-backup"],
          "manager_member_ids": ["staff-lead"],
          "autonomous": true
        },
        "stage_action": {
          "stage_id": "code_review",
          "capability_id": "code_review"
        },
        "available_stage_actions": [
          {
            "capability_id": "code_review",
            "label": "代码审核",
            "supported_roles": ["review_ready", "review"],
            "requires_workflow_uuid": false
          },
          {
            "capability_id": "saved_workflow",
            "label": "Saved Workflow",
            "supported_roles": ["review_ready", "review", "merging"],
            "requires_workflow_uuid": true
          }
        ]
      }
    }
  ]
}
```

客户端展示以上字段，不在本地重算 status role、负责人或阶段动作。

## 开发生命周期：`dev_lifecycle`

开发生命周期适合代码任务。代码审核和合并是内置阶段能力；项目未显式配置时，gateway 读模型会为
`dev_lifecycle` 补齐 `code_review` 和 `git_merge` 默认阶段动作。示例中显式写出这些动作，便于
看清配置字段。

```toml
[task_board_settings]
workflow_id = "dev_lifecycle"
max_concurrent = 2

[[staff.project_state.stage_assignments]]
stage_id = "planning"
primary_member_id = "staff-planner"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "running"
primary_member_id = "staff-executor"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "code_review"
primary_member_id = "staff-reviewer"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "pr_submitted"
primary_member_id = "staff-merger"
autonomous = true

[[staff.project_state.stage_actions]]
stage_id = "code_complete"
capability_id = "code_review"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "code_review"
capability_id = "code_review"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "pr_submitted"
capability_id = "git_merge"
metadata.gateway_command = "merge"
```

`StatusRole::ReviewReady` / `StatusRole::Review` 默认进入代码审核，`StatusRole::Merging`
默认进入合并。worktree 池只提供执行 workspace 和 merge lock，不挑选任务。

## 营销活动执行：`marketing_campaign`

营销活动用业务阶段替代代码阶段。策划和投放可以走默认执行；复盘和结案通常通过
`saved_workflow` 阶段能力绑定已保存 Workflow，避免复用代码审核/合并语义。

```toml
[task_board_settings]
workflow_id = "marketing_campaign"
max_concurrent = 3

[[staff.project_state.stage_assignments]]
stage_id = "campaign_planning"
primary_member_id = "marketing-planner"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "campaign_running"
primary_member_id = "campaign-operator"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "campaign_review_ready"
primary_member_id = "campaign-reviewer"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "campaign_closing"
primary_member_id = "campaign-owner"
autonomous = true

[[staff.project_state.stage_actions]]
stage_id = "campaign_review_ready"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000101"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "campaign_closing"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000102"
metadata.gateway_command = "run_stage_action"
```

`campaign_review_ready` 成功后沿 `succeed` 边进入 `campaign_reviewing`；
`campaign_closing` 成功后进入 `campaign_done`。失败时走当前 workflow 的 `failed` 边；
没有失败边则暂停并记录原因。

## 招聘流程：`recruiting_pipeline`

招聘流程可把初筛、面试、评审和 Offer 跟进分别交给 `saved_workflow` 阶段能力。

```toml
[task_board_settings]
workflow_id = "recruiting_pipeline"
max_concurrent = 4

[[staff.project_state.stage_assignments]]
stage_id = "recruiting_screening"
primary_member_id = "recruiter-screening"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "recruiting_interviewing"
primary_member_id = "interview-coordinator"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "recruiting_review_ready"
primary_member_id = "hiring-committee"
autonomous = true

[[staff.project_state.stage_assignments]]
stage_id = "recruiting_offer"
primary_member_id = "recruiting-owner"
autonomous = true

[[staff.project_state.stage_actions]]
stage_id = "recruiting_screening"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000201"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "recruiting_interviewing"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000202"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "recruiting_review_ready"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000203"
metadata.gateway_command = "run_stage_action"

[[staff.project_state.stage_actions]]
stage_id = "recruiting_offer"
capability_id = "saved_workflow"
workflow_uuid = "00000000-0000-4000-8000-000000000204"
metadata.gateway_command = "run_stage_action"
```

这些阶段由 `StatusRole` 决定是否可启动。`saved_workflow` 请求只传任务摘要、阶段 key、
员工信息和优先级；不要传密钥、完整 prompt、日志全文、diff 或 worktree 路径。

旧 `staff.project_state.stage_workflow_bindings` 仍可读取，并会投影成 `saved_workflow`
阶段动作；新示例优先使用 `staff.project_state.stage_actions`。

## Agent Scheduler 与资源边界

Agent Scf






heduler 负责任务注册、状态同步、worker/outbox 分发和命令处理。生命周期定义来自
`vw-agent` lifecycle API；桌面端写入 `workflow_id`、状态 key 与必要阶段 metadata，
gateway 读模型优先返回这些 key。

task-pool worktree 池只负责 workspace、槽位租约和 merge lock。它不是调度器，不计算
`marketing_campaign` 或 `recruiting_pipeline` 的候选阶段，也不推进任务状态。