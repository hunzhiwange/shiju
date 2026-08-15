# 任务 4: 处理器与 Gateway 提交编排

批次：【批次 3】 依赖批次 1,2

状态：未开始

目的：移除处理器直接写 raw，由 Gateway 聚合工件并在 UI 保存时交给原子提交边界。

来源任务：无

预计会话范围：覆盖一条完整写入链：处理器产出、Gateway 暂存、UI save 提交与确认；不修改桌面轨迹展示。

## 文件

- 修改：`crates/vw-agent/src/session/processor/artifacts.rs`
- 修改：`crates/vw-agent/src/session/processor/types.rs`
- 修改：`crates/vw-agent/src/session/processor/runner.rs`
- 测试：`crates/vw-agent/src/session/processor/artifacts_tests.rs`
- 测试：`crates/vw-agent/src/session/processor/runner_tests.rs`
- 新建：`crates/vw-agent/src/gateway/api/handlers/session/raw_commit.rs`
- 新建：`crates/vw-agent/src/gateway/api/handlers/session/raw_commit_tests.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/session.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/session/stream.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/session/ui_ops.rs`
- 测试：`crates/vw-agent/src/gateway/api/handlers/session/stream_tests.rs`
- 测试：`crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs`

## 上下文

- 将 `persist_llm_raw_step_payload` 改为纯构造函数；新增内部 `StreamEvent::LlmRawArtifact`，必须先于对应 `StepFinish` 交给 Gateway，且不序列化给客户端。
- Gateway 用预分配助手消息 ID 作为 `turn_id`；暂存注册表以 `(tenant, project, session, turn)` 隔离，并为每次快照分配版本。
- `session_ui_save` 获取待提交版本并调用任务 3 的原子 API；仅成功后确认清理，失败或并发新增 round 时不得丢数据。
- `persist_stream_chat_turn_for_agent` 的错误不能再被 `Err(_) => (None, None)` 吞掉。

## 测试计划

- 行为：raw 在 StepFinish 前进入待提交区；失败保存保留工件；成功保存一次性使步骤和 raw 可见；后续 turn 的相同 step index 不覆盖前一 turn。
- 失败验证测试：替换旧 `raw_payload_is_visible_before_step_finish` 和 `raw_payload_survives_rejected_step_finish`，新增 `raw_artifact_is_buffered_before_step_finish`、`failed_commit_retains_pending_raw`、`successful_commit_acknowledges_only_claimed_version`。
- 失败验证命令：`cargo test -p vw-agent raw_artifact_is_buffered_before_step_finish --lib`
- 预期失败原因：处理器当前直接写数据库，Gateway 没有待提交工件或版本确认机制。
- 通过验证命令：`cargo test -p vw-agent raw_artifact --lib && cargo test -p vw-agent session_ui_save --lib && cargo test -p vw-agent persist_stream_chat_turn --lib`
- 模拟策略：LLM provider 保持现有测试替身；注册表和 SessionStore 使用真实类型，数据库失败使用任务 3 的故障方式。

## 步骤

1. 先改写 runner 测试，证明事件顺序与“处理器不落库”的新边界。
2. 为待提交注册表添加失败保留、成功精确确认和作用域隔离测试。
3. 将助手消息 ID 注入处理请求的 raw turn 上下文，并逐 round 构造差量工件。
4. Gateway 消费内部 raw 事件；UI 保存 handler 以 claim/commit/ack 顺序调用原子 API。
5. 删除 raw 的 warn-and-continue 写路径；消息持久化失败返回终止错误，保留足够上下文日志但不记录敏感载荷。
6. 运行聚焦测试、stream 周边测试与 clippy。

## 验证

- 运行：`cargo test -p vw-agent raw_artifact --lib`
- 运行：`cargo test -p vw-agent session_ui_save --lib`
- 运行：`cargo test -p vw-agent persist_stream_chat_turn --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：处理器零数据库 raw 写入；Gateway 失败不丢待提交数据；成功提交后无孤立步骤或 raw；保存错误不产生 Done 假成功。
- 所需证据：新测试先失败后通过、旧直接持久化调用已从生产路径移除、相关测试和 clippy 退出码 0。

## 完成

确认处理器、Gateway 注册表、原子 save handler 和错误终止语义均有测试证据后才可标记完成。
