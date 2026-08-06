# 目标模式与会话控制 计划

目标：输入栏将「目标模式」放到完全访问右侧并默认开启；「会话控制」拆成代理/工具/技能三个 TAB 入口；目标模式改为可与工作流并存的「不达目的不罢休」续轮，不再等同严格工作流。

范围：调整 `vw-desktop` 底部栏与会话选择器入口；解除目标/工作流互斥并默认开启；修正流路由：仅工作流进入 `task_plan_run`，目标叠加验收续轮；无工作流时走轻量目标续轮（对齐 Claude/Codex `/goal`）；同步 CLI 双开关；补聚焦测试与 clippy。

范围外：不做 Codex 完整 pause/resume/budget 生命周期 UI；不重做 create-task-plans 技能本身；不改权限模型；不迁移旧会话持久化字段；不实现主题/支付等无关功能。

假设：用户给出的 transcript `7cc343c5-…` 实为系统主题计划，与本需求无关；语义以 Claude Code `/goal` 与 Codex Goals 文档为准——独立评估器判断完成条件，未达成则自动续轮，且与 plan/workflow 可并存。

## 设计决策

- 底部栏顺序：`完全访问 | 目标模式 | 工作流 | 子智能体 | 代理 | 工具 | 技能 | …`；原单一「会话控制」改为三个 TAB 按钮，点击即选中对应 `SessionToolSelectorTab` 并打开同一 popover，popover 内仍保留 TAB。
- 目标模式默认 `true`；与工作流互不关闭；plan-run 子会话仍强制关闭目标模式（沿用现有 `open.rs` 约束）。
- 流路由：`workflow_mode_enabled` 才进入 `task_plan_run`；`workflow && goal` → `TaskPlanRunMode::Goal`；`workflow && !goal` → `Workflow`；`!workflow && goal` → 普通聊天 + 独立验收续轮，不强制建计划目录。
- 轻量目标续轮以用户本轮查询为完成条件，复用/抽取 `goal.rs` 评估 JSON 契约与续轮提示，不要求编号任务文件全部删除才算完成。

## 架构说明

- UI：`input_panel/mod.rs`、`send_controls.rs`、`tool_selector.rs`、popover 消息。
- 开关：`task_mode.rs`、`SessionRuntimeState`、CLI `session_mode`/`settings_panel`。
- 执行：`agent_stream.rs` 路由；`vw-agent` 目标评估/续轮；现有 Goal plan-run 仅在工作流开启时使用。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 仓库默认不跑全量 `cargo test`；验证以聚焦测试命令（用户已要求本计划执行时允许）与 `cargo clippy --all-targets --all-features -- -D warnings` 为准。
- 将实施步骤限制在失败测试所证明的行为范围内。
