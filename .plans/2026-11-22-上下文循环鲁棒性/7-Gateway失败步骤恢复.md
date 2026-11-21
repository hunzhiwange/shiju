# 任务 7　Gateway 仅恢复超限的模型步骤

先阅读并遵守 [公共执行规则](plan.md)。

**批次**　3。**依赖**　任务 4。

**状态**　未开始

**目的**　成功压缩并在原工具循环内重试失败采样，恢复任务 1 暂停的工具后续跑。

**来源**　LLM 上下文循环静态审查。

## 文件与写入顺序

- `crates/libs/agent-loop/src/query_engine.rs`
- `crates/libs/agent-loop/src/query_engine_host.rs`
- `crates/libs/agent-loop/src/core/tool_loop.rs`
- `crates/libs/agent-loop/src/core/turn.rs`
- `crates/vw-agent/src/agent/loop_/query_engine.rs`
- `crates/libs/agent-loop/src/query_engine_tests.rs`

任务 9 随后复用同一采样入口做主动预检。 本任务只负责上述行为及其直接调用接线。

## 上下文

任务 1 提供重放边界，任务 4 提供已确认原始增量。当前失败处理仍从 QueryEngine 外层返回。

## 失败验证与验收

- 先在现有测试文件增加或更名为 `query_engine_step_recovery_` 前缀的用例。第一步调用计数工具，第二步超限，摘要返回可缩减结果，第三步完成。任务 1 的保护生效后该场景仍无法继续。添加二次超限或无缩减场景。
- 失败和通过验证同用 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib query_engine_step_recovery_`，确认有效测试数。
- 首步回归运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib prompt_too_long_compacts_and_retries_once`。
- 完成证据包括验证命令和退出码、有效测试数、修复前的预期断言或故障分支，以及修复后的以下断言。第三次模型请求携带原调用 ID 与结果，工具仍只执行一次；同一步至多一次压缩恢复，无进展时报错。

## 实施

将恢复放在同一个模型采样边界。按已授权的工具快照和已确认历史构造缩减后的请求，保存技能及该步已消费的 Hook 上下文，复用当前 turn 身份、取消令牌和去重状态。真实模型超限且本次采样可重试时才执行。旧的首步恢复也走这条入口，移除外层整轮重试。另运行原首步恢复用例。
