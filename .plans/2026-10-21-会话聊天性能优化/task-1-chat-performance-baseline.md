# 任务 1：建立聊天性能基线与回归探针

## 状态

未开始

## 依赖

无；批次 1。

## 目标

建立可重复的长会话与长流式回复工作负载，记录当前高度同步、完整渲染缓存构建和真实聊天壳渲染的基线。探针只提供诊断数据和语义断言，不用机器相关耗时阈值制造不稳定测试。

## 文件范围

- 修改 `crates/vw-desktop/src/app/components/chat_panel/mod.rs`
- 新增 `crates/vw-desktop/src/app/components/chat_panel/performance_tests.rs`

## 实现要求

1. 在独立测试文件中建立确定性 fixture：1,000 条混合历史消息、128 KiB 逐步增长的 assistant 回复、中文 Markdown、代码围栏、思考块和工具块。
2. 增加一个默认忽略、可显式执行的 `chat_performance_probe`：分别记录同长度虚拟器同步、增长正文的完整渲染缓存构建、聊天壳真实渲染的总耗时和操作次数。
3. 使用 `std::hint::black_box` 防止诊断循环被优化；输出稳定的字段名，便于优化前后人工比较。
4. 探针必须断言最终可见文本、消息数与渲染块语义正确，但不得断言固定毫秒数。
5. 复用现有消息构造和渲染入口，不新增生产依赖、不修改生产行为。
6. 记录优化前样本时注明机器与构建模式；同一执行会话完成后续对比，避免跨机器比较。

## 测试先行说明

本任务只增加行为中立的诊断工具，不改变产品行为，因此不要求先提交失败测试。探针的语义断言必须从首次提交起通过；后续任务使用它做 before/after 对比，并各自增加确定性失败测试。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo test -p vw-desktop conversation_shell_renders_visual_regression_scenarios_without_panic -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时在本文件记录探针输出、fixture 规模、真实渲染场景结果和完整 clippy 结果；不得用删除语义断言或放宽 fixture 规模来获得通过。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-1-chat-performance-baseline.md`。

### 任务边界

只建立性能基线和回归探针，不实施性能修复。先阅读 `AGENTS.md`、计划总览、`chat_panel/mod.rs`、`virtualizer.rs`、`message_view/render_cache.rs` 及相邻测试。复用现有私有入口；若测试模块无法访问某入口，优先把探针放到最靠近该入口的独立测试模块，不要为测试扩大生产 API。

### 必须遵守

1. 严格遵循测试驱动开发：本任务属于行为中立诊断，先写语义断言，再补齐探针；后续修复任务必须先看到对应断言失败。
2. 只修改本任务文件范围；如果编译所需的测试模块声明必须增加，只改列出的 `mod.rs`。
3. 不引入新依赖，不改变生产路径，不设置固定时间阈值。
4. 测试文件必须独立于业务实现，名称按行为与结果命名。
5. 中文、代码、路径和动态文本 fixture 必须覆盖实际渲染路径；中文不得使用 `Font::MONOSPACE`。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试并保留关键输出；不得通过删除或跳过语义断言掩盖问题。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

