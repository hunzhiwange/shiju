# 任务 9　Gateway 发请求前预算覆盖完整载荷

先阅读并遵守 [公共执行规则](plan.md)。

**批次**　4。**依赖**　任务 7。

**状态**　未开始

**目的**　在模型步发送前识别消息、工具 schema 和图片造成的溢出。

**来源**　LLM 上下文循环静态审查。

## 文件与写入顺序

- `crates/libs/agent-loop/src/query_engine.rs`
- `crates/libs/agent-loop/src/core/tool_loop.rs`
- `crates/libs/agent-loop/src/core/llm_round.rs`
- `crates/libs/session-engine/src/context_governance.rs`
- `crates/libs/agent-loop/src/query_engine_tests.rs`

必须使用任务 7 的采样边界，不能把恢复挪回外层。 本任务只负责上述行为及其直接调用接线。

## 上下文

QueryEngine 初检只估算历史文本，后续才组装动态指令、暴露工具及多模态内容。

## 失败验证与验收

- 先在现有测试文件增加或更名为 `gateway_context_budget_` 前缀的用例。分别构造长工具 schema、固定开销超过窗口、工具输出推动后续请求超窗。记录脚本化 Provider 看到的载荷。原实现会直接发送至少一个超窗输入。
- 失败和通过验证同用 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib gateway_context_budget_`，确认有效测试数。
- 运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib query_engine_step_recovery_` 检查该步骤的响应式恢复仍有效。
- 完成证据包括验证命令和退出码、有效测试数、修复前的预期断言或故障分支，以及修复后的以下断言。可压缩的输入在首次发送前缩减，无法缩减的固定开销明确失败；不重复执行工具，禁用压缩行为符合现有配置。

## 实施

复用桌面预算入口，在 Provider 请求就绪时计入系统、消息、工具约束和图片成本并预留输出。文本与原生工具说明各计一次。压缩后重新估算，把主动预检与供应商溢出兜底挂在任务 7 的同一步；沿用同一工具和 Hook 上下文快照。再跑任务 7 的过滤条件。
