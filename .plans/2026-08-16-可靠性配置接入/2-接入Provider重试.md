# 任务 2: 接入 Provider 重试

批次：【批次 2】 依赖批次 1

状态：未开始

目的：让 `provider_retries` 和 `provider_backoff_ms` 统一控制 session processor 发起的模型请求重试与退避。

来源任务：无

预计会话范围：一条会话请求管道行为变更，涵盖所有生产请求构造者、主请求/内部压缩/Todo 更新调用和相邻测试；不改变错误是否可重试的分类。

## 文件

- 修改：`crates/vw-agent/src/session/processor/types.rs`
- 修改：`crates/vw-agent/src/session/processor/runner.rs`
- 修改：`crates/vw-agent/src/session/processor/llm_runner.rs`
- 修改：`crates/vw-agent/src/session/processor/todo_updates.rs`
- 修改：`crates/vw-agent/src/session/processor/llm_runner_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/runner_tests.rs`
- 修改：`crates/vw-agent/src/session/processor/types_tests.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/session/stream.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/session/stream_tests.rs`
- 修改：`crates/vw-agent/src/channels/manager/message_execution.rs`
- 修改：`crates/vw-agent/src/channels/manager/message_execution_tests.rs`
- 修改：`crates/vw-agent/src/acp/backend.rs`
- 修改：`crates/vw-agent/src/acp/backend_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/runner.rs`
- 修改：其他构造 `session::processor::Request` 的相邻测试 fixture，以 `rg` 实际结果为准

## 上下文

- `run_llm_step_with_retry` 当前接收总尝试次数，并使用硬编码 `300ms` 起始、`5000ms` 上限的退避。
- `runner.rs` 当前按 ACP/Todo 状态硬编码 2/3/5 次尝试；这是 `provider_retries` 未生效的根因。
- 当前 `runner.rs`、`types.rs`、Gateway stream 文件均有用户未提交改动，必须小块编辑。

## 测试计划

- 行为：`provider_retries = N` 产生最多 `N + 1` 次请求；可重试失败按 `provider_backoff_ms` 起始指数退避；不可重试错误仍立即停止。
- 失败验证测试：在 `llm_runner_tests.rs` 新增 `configured_provider_retries_control_total_attempts` 和 `configured_provider_backoff_controls_retry_delay`；在 `runner_tests.rs` 新增请求可靠性值传入主模型步骤的行为测试。
- 失败验证命令：`cargo clippy -p vibe-agent --all-targets --all-features -- -D warnings`
- 预期失败原因：新测试要求可配置的重试语义和退避，而现有函数签名及硬编码策略无法满足。
- 通过验证命令：`cargo clippy -p vibe-agent --all-targets --all-features -- -D warnings`
- 模拟策略：复用现有确定性 scripted LLM stream 边界记录尝试次数；对退避计算提取纯函数并直接验证计算结果，不用真实长时间 sleep。仓库禁止默认运行 `cargo test`，所以用 all-targets clippy 编译测试目标并在交接中说明未执行测试。

## 步骤

1. 先更新测试，明确“重试次数”与“总尝试次数”的差异，并运行失败验证。
2. 向 `processor::Request` 增加强类型 `ReliabilityConfig`，更新所有生产构造者和 fixture；Gateway 按 tenant 与 project root 加载有效配置，Channel 使用已有 runtime context 配置。
3. 将 `run_llm_step_with_retry` 参数改为明确的“额外重试次数 + 初始退避”，使用饱和指数退避并保留上限。
4. 在主模型步骤、内部压缩与 Todo 更新中使用同一请求级可靠性配置，删除 ACP/Todo 硬编码重试预算。
5. 检查 `session::llm::stream` 中 `max_retries`/`retries` 的现有内层语义，确保默认不与 processor 外层重复重试；如需修正零重试边界，增加对应回归断言。
6. 运行通过验证，复查所有 `processor::Request` 构造者已覆盖且未修改错误分类规则。

## 验证

- 运行：`rg -n 'retry_budget = if|300u64\.saturating_mul' crates/vw-agent/src/session/processor`
- 预期：无旧硬编码重试预算或固定初始退避残留。
- 运行：`cargo clippy -p vibe-agent --all-targets --all-features -- -D warnings`
- 预期：退出码 0，所有请求构造者和测试目标编译成功。
- 所需证据：失败阶段输出，重试次数/退避计算断言，旧硬编码零残留搜索，clippy 退出码和已审查 diff。

## 完成

完成后，`provider_retries` 必须影响桌面聊天、Gateway、Channel、Task Plan 和 ACP 进入的 session processor 模型请求，`provider_backoff_ms` 必须控制它们的初始重试等待。

