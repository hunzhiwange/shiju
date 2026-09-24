# Thread 命令评测计划

目标：以真实网关评测验证模型能创建用户可见任务，并能按名称调用其余 thread 命令。

范围：修复首次网关请求中阻碍 `create_thread` 的父任务时序与系统提示条件；扩展现有 Promptfoo 轨迹评分，新增包含“写一篇50字的科幻小说”的 thread 命令评测场景。

范围外：跨项目创建、Codex 项目 ID 互通、真正执行 `handoff_thread`、全量评测矩阵或旧 SessionTool 入口恢复。

假设：`/v1/chat/stream` 返回 `chat.tool` 和 `chat.done`；Provider 已记录工具名、输入、输出、状态与会话 ID；`handoff_thread` 当前明确报不支持。

## 设计决策

- 用 Promptfoo 验证模型选择及真实工具轨迹，用 Rust 聚焦测试定位注册、提示和持久化错误。仅靠回答提及工具名不能算通过。
- 方案一是单轮提示要求全部命令，依赖模型一次完成长链；方案二是同一父会话分阶段评测，便于定位每条命令。采用方案二。
- `handoff_thread` 的预期结果是明确“不支持”，不作为工具执行成功计分；其余命令按成功状态及真实结果评分。

## 架构说明

- 系统提示由 `session/system.rs` 按注册工具名组装；网关流在 `gateway/api/handlers/session/stream.rs` 建立父会话；thread 工具在 `tools/thread_tool/mod.rs`。
- Promptfoo 入口为 `evals/promptfoo/promptfooconfig.yaml`，Provider 解析 SSE，`assertions.mjs` 评分。评测只使用当前项目，保留租户、项目、目录、父子边界。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
