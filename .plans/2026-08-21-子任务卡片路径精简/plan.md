# 子任务卡片路径精简计划

目标：让 plan-run 子任务卡片主行只保留任务标题、状态和动作，完整任务路径仅在 hover tooltip 中展示。

范围：覆盖聊天流中的 `SubtaskStarted` 与 `SubtaskFinished` workflow 卡片；隐藏主行输出摘要中的任务文件路径，复用紧凑宽度，并用现有 `metadata.task_path` 构建路径 tooltip。

范围外：不改任务文件落盘、plan-run 事件协议、执行输出详情、子会话打开/停止行为、普通 workflow 节点与其它工具卡片。

假设：上一份 `.plans/2026-08-21-规划任务卡片精简/` 先执行；现有子任务事件已把完整 `task_file` 写入 `metadata.task_path`，详情按钮仍可查看原始输出。

## 设计决策

- 选择在 `workflow_view` 的 plan-run 展示分支中识别 `subtask_started`/`subtask_finished`，主行不再渲染通用 output summary；不修改 `plan_run_event_output_text`，以保留详情弹窗信息。
- tooltip 只展示非空完整路径，格式为 `任务路径：<task_path>`；缺少路径时不构建 tooltip，不回退到项目目录或 session ID。
- 子任务卡片复用上一任务建立的 plan-run 内容收缩与最大宽度约束，不改变共享 `tool_activity_header` 或普通 workflow 卡片。

## 架构说明

- `agent_stream.rs` 已负责传递子任务路径；本任务只在 `workflow_view.rs` 的展示层改变主行摘要、tooltip 与局部布局。
- tooltip 使用默认字体与可换行文本，兼容中文路径和暗黑主题，避免长路径重新撑宽卡片。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实 workflow tool payload 与 Iced headless 布局，不验证模拟对象或私有实现细节。
