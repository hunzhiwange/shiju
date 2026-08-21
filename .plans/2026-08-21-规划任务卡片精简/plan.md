# 规划任务卡片精简计划

目标：让聊天流中的计划任务卡片仅展示简短状态文案，按内容收缩宽度，并在 hover tooltip 中展示 UUID 与任务路径。

范围：仅调整 `vw-desktop` 中 `workflow_node` 的 plan-run 规划事件元数据和聊天卡片渲染；规划中显示“正在规划计划任务”，计划创建后显示“计划任务已创建，共 x 个任务。”，tooltip 显示 UUID 和任务路径。

范围外：不改 plan-run 调度、SSE 协议、任务计划落盘位置、子会话打开/停止行为、其它工具卡片和 input panel 子代理弹层。

假设：`PlanningStarted` 和 `PlanCreated` 继续共用同一 planning tool call ID；现有 `metadata.task_id` 与 `metadata.task_path` 可作为 tooltip 数据源，`PlanCreated.subtask_count` 是创建完成文案的权威数量。

## 设计决策

- 选择在 `workflow_view` 中对 plan-run planning 卡片做局部分支：按事件类型生成两种固定文案，不再把 UUID 和输出预览塞进主行。
- 为 `PlanCreated` 元数据显式补充 `subtask_count`，不从展示文案或 JSON 输出反向解析数量。
- 规划卡片使用内容收缩与局部最大宽度；不改共享 `tool_activity_header` 的 `Length::Fill`，避免影响 Bash、文件、MCP 等其它工具卡片。
- tooltip 使用两行默认字体文本，格式为 `UUID：<task_id>` 和 `任务路径：<task_path>`；缺失值时只显示已知项，两者都缺失时不构建 tooltip。

## 架构说明

- `agent_stream.rs` 负责把强类型 plan-run 事件转为 `workflow_node` 元数据；`workflow_view.rs` 只负责从结构化元数据派生文案、tooltip 与布局。
- 中文标签、UUID 和路径 tooltip 不使用 `Font::MONOSPACE`，需要通过 headless Iced 布局覆盖非 ASCII 整形路径。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实元数据转换与 Iced 布局，不验证模拟对象或私有实现细节。
