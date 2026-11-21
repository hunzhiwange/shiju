# 任务 1　Gateway 工具已开始后停止整轮重放

先阅读并遵守 [公共执行规则](plan.md)。

**批次**　1。**依赖**　无。

**状态**　未开始

**目的**　禁止已有工具执行、执行状态未知或已有不可重放输出时，从旧历史重新进入工具循环。

**来源**　LLM 上下文循环静态审查。

## 文件与写入顺序

- `crates/libs/agent-loop/src/query_engine.rs`
- `crates/libs/agent-loop/src/core/tool_loop.rs`
- `crates/libs/agent-loop/src/core/tool_execution.rs`
- `crates/libs/agent-loop/src/query_engine_host.rs`
- `crates/libs/agent-loop/src/core/turn.rs`
- `crates/vw-agent/src/agent/loop_/query_engine.rs`
- `crates/libs/agent-loop/src/query_engine_tests.rs`

任务 4 和 7 先后接续 Gateway 文件。 本任务只负责上述行为及其直接调用接线。

## 上下文

`submit_message_in_project` 当前仅按超限错误判断是否再调用 `agent_turn`。

## 失败验证与验收

- 先在现有测试文件增加或更名为 `query_engine_no_turn_replay_` 前缀的用例。脚本化模型先请求一个计数工具，工具完成后返回超限。故障后预置第三次请求可再次要求相同工具，旧代码会发起第三次请求。另验证已进入派发但未写入结果的状态不能依据历史长度判定为安全。
- 失败和通过验证同用 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib query_engine_no_turn_replay_`，确认有效测试数。
- 运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-agent --lib prompt_too_long_compacts_and_retries_once`。
- 完成证据包括验证命令和退出码、有效测试数、修复前的预期断言或故障分支，以及修复后的以下断言。故障返回，模型请求恰好两次，工具只执行一次。原有首步超限仍可恢复。

## 实施

在真实工具派发前记录本轮已经跨过可重放边界，并将该状态传回 Gateway 调用方。使用显式状态覆盖尚未写回历史的工具；排除供应商明确禁止重放及取消等路径。保留无工具且可安全重放的首步恢复。同步直接宿主和夹具的签名；按文件范围只改这道判定。
