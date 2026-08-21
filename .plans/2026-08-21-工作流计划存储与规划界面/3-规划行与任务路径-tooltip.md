# 任务 3: 规划行与任务路径 tooltip

批次：【批次 3】 依赖批次 1,2

状态：已完成，待回归

目的：将规划子会话压缩为 `规划 · <UUID> · 计划任务` 一行，并在 hover 时展示真实计划目录或任务文件路径。

来源任务：用户提出“规划 UI 优化；规划 · uuid · 计划任务，hover 显示任务路径，不需要展示那么多东西”。

预计会话范围：沿 plan-run 事件、workflow_node 元数据、子会话投影到 input panel 行组件贯通一个可选任务路径字段，并只精简工作流规划行。

## 文件

- 修改：`crates/vw-desktop/src/app/impls/agent_stream.rs`
- 修改：`crates/vw-desktop/src/app/impls/agent_stream_tests.rs`
- 修改：`crates/vw-desktop/src/app/session/agent_children.rs`
- 修改：`crates/vw-desktop/src/app/session/agent_children_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/agent_sessions.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/agent_sessions_tests.rs`

## 上下文

- `PlanningStarted/PlanCreated` 已携带 `plan_dir`，`SubtaskStarted/SubtaskFinished` 已携带 `task_file`，但 `PlanRunSessionControl` 目前只保留 task/session/status。
- `AgentChildSession.directory` 是打开会话所需的项目目录，不能改成任务路径；应新增独立可选字段。
- 后续 `SessionDelta` 不含路径，合并最新 control 时必须保留同 session 已知路径。

## 测试计划

- 行为：规划行只显示一行固定文案；hover 是用户级 plan 目录，执行子任务 hover 是编号任务文件；打开会话仍使用项目目录。
- 失败验证测试：新增 `plan_run_tool_block_exposes_plan_or_task_path`、`plan_run_controls_preserve_known_task_path_across_deltas`，改写 `workflow_planning_child_uses_compact_label_and_task_path_tooltip`。
- 失败验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期失败原因：测试先断言 `task_path` 字段和路径元数据，现有结构不存在该字段，产生编译失败。
- 通过验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 模拟策略：使用真实事件 DTO、tool block 文本解析和 Iced Element 构建；不模拟字体整形，实际渲染另做手工验收。

## 步骤

1. 为 plan-run 事件提取可选任务路径，并写入 `workflow_node.metadata.task_path`。
2. 扩展 `PlanRunSessionControl` 与 `AgentChildSession`，在单消息和跨消息合并时保留最近一次非空路径。
3. 保持 `directory` 专用于 `OpenAgentSession.project_path`，tooltip 优先使用 `task_path`；普通 agent 保留现有目录 tooltip。
4. 为 `WorkflowPlanning` 渲染单行默认字体文本 `规划 · <UUID> · 计划任务`，移除该行的 agent badge、重复状态和第二行任务文案；停止按钮行为不变。
5. UUID 与中文保持同一默认字体，不使用 `Font::MONOSPACE`；沿用 4px 滚动条与暗黑主题可读样式。
6. 运行 clippy；再运行桌面进行浅色、暗色实际 hover 与非 ASCII 字形整形检查。

## 验证

- 运行：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 运行：`cargo run -p vw-desktop --bin shi-ju`
- 预期：clippy 退出码 0；规划行单行紧凑、hover 路径准确、停止/打开会话不回归，浅色和暗色均无低对比或字形 panic。
- 所需证据：失败验证编译错误、clippy 退出码、路径跨 delta 保留断言、实际渲染截图或检查记录、停止与打开消息保持原 project path 的断言。

## 完成

- 事件元数据到 UI 投影全链路具有独立 `task_path`，规划行满足精简文案且 tooltip 不再误用项目目录。
- 若无法进行实际 GUI 渲染检查，状态必须为 `已完成，待回归` 并记录缺失证据。

## 验证结果

- 失败验证：聚焦 clippy 退出码 101；7 处编译错误证明 `task_path` 字段与 tooltip helper 尚不存在。
- 通过验证：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings` 退出码 0。
- 全量门禁：`cargo clippy --all-targets --all-features -- -D warnings` 退出码 0；`git diff --check` 退出码 0。
- 路径与交互断言：新增测试覆盖 plan 目录/编号任务文件元数据、跨 delta 保留、tooltip 优先 `task_path`，并断言打开会话仍使用项目 `directory`；停止消息逻辑未变。
- 启动检查：`cargo run -p vw-desktop --bin shi-ju` 成功启动，未见非 ASCII 字形整形 panic；因本环境 `screencapture` 返回 `could not create image from display`，无法完成真实规划行的浅色/暗色 hover 截图验收。
- 待回归：仓库禁令要求不执行 `cargo test`，新增行为测试仅由 all-targets clippy 证明可编译；实际浅色/暗色 hover 仍需在有显示环境中手工验收。
