# 任务 2: 稳定轮次与增量 raw 模型

批次：【批次 1】 无

状态：未开始

目的：定义稳定轮次身份和可无损重建的 raw 差量模型，消除工具轮次的公共上下文复制。

来源任务：无

预计会话范围：仅新增纯数据模型、差量编码和重建逻辑，以及 `ChatSessionStep` 的轮次引用；不接数据库和网络。

## 文件

- 新建：`crates/vw-agent/src/session/llm_raw.rs`
- 新建：`crates/vw-agent/src/session/llm_raw_tests.rs`
- 修改：`crates/vw-agent/src/session/mod.rs`
- 修改：`crates/vw-shared/src/session/ui_types.rs`
- 测试：`crates/vw-shared/src/session/ui_types_tests.rs`

## 上下文

- 新模型须定义 `LlmRawTurnContext`、`LlmRawRound`、`LlmRawArtifact` 和纯函数重建接口。
- turn 公共记录只含 `turn_id`、session、system、工具目录、初始请求配置与初始 provider messages；round 记录含 `step_index`、前缀保留长度、消息/来源后缀、本轮模型配置差异和唯一输出。
- `ChatSessionStep` 增加可序列化的 `turn_id`；工具时间仍只存在 `tool_timings`，raw 类型不得复制时间。

## 测试计划

- 行为：两个不同 turn 可复用同一 step index；同一 turn 的工具后请求只保存差量，并可逐步无损重建当前完整 raw JSON。
- 失败验证测试：新增 `raw_turn_key_distinguishes_reused_step_indices`、`raw_rounds_reconstruct_provider_messages_after_tools`、`raw_round_delta_handles_compacted_prefix`。
- 失败验证命令：`cargo test -p vw-agent llm_raw --lib`
- 预期失败原因：仓库尚无稳定 turn 键、差量类型和重建器。
- 通过验证命令：`cargo test -p vw-agent llm_raw --lib && cargo test -p vw-shared ui_types --lib`
- 模拟策略：全部使用纯 serde 值和真实差量算法，不使用 mock。

## 步骤

1. 先写复用 step index、工具后追加和上下文压缩三类失败测试。
2. 运行失败验证命令并确认缺少模型或行为不满足。
3. 实现最长公共前缀差量；非纯追加时以 `prefix_len + suffix` 表达替换，保证可重建。
4. 为 `ChatSessionStep` 增加 `turn_id`，并验证序列化往返。
5. 检查 raw 数据结构不含工具起止时间，再运行聚焦测试和 clippy。

## 验证

- 运行：`cargo test -p vw-agent llm_raw --lib`
- 运行：`cargo test -p vw-shared ui_types --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：所有重建值与原始请求逐字段相等；不同 turn 不冲突；raw 不含工具 timing；clippy 无警告。
- 所需证据：三个新增测试先失败后通过、序列化往返通过、命令退出码均为 0。

## 完成

确认新 raw 模型、差量算法、重建接口及 `ChatSessionStep.turn_id` 已由聚焦测试覆盖后才可标记完成。
