# 任务 7：完成长会话真实渲染回归验收

## 状态

未开始

## 依赖

依赖任务 5、6；批次 5。

## 目标

把前置任务的复杂度保证固化为端到端回归测试，并通过真实 Iced 文本整形/布局路径验证暗黑主题、中文、Markdown、思考块、工具块和长流式消息不会 panic 或退化回全文热路径。

## 文件范围

- 修改 `crates/vw-desktop/src/app/components/chat_panel/performance_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui_tests.rs`

## 实现要求

1. 将任务 1 探针扩展为优化后诊断报告，保留相同 fixture 和字段名，输出 before/after 可直接比较的数据。
2. 增加端到端确定性断言：1,000 条历史、128 KiB assistant 流、数百 delta 下，同长度索引完整重建为 0、流式期间全文解析为 0、完成时规范化全文解析为 1、单块预热捕获消息不超过 32。
3. 最终 transcript 与一次性规范解析在 Markdown、中文、代码围栏、思考块和工具块上等价。
4. 扩展现有聊天壳视觉回归场景，实际执行 Iced renderer 的文本整形、布局与 draw 路径，而不只构建 `Element`；覆盖暗黑与明亮主题、中文动态文本、弹窗附近状态和长消息滚动窗口。
5. 断言虚拟窗口创建的消息视图数量受 viewport/overscan 限制，不随 1,000 条历史线性增长。
6. 不使用固定毫秒阈值作为 CI 成败条件；诊断耗时与操作计数一并记录，正确性由操作计数和语义等价保证。

## 测试先行说明

这是最终验收任务，不新增生产行为。先写完整验收断言；若任一断言失败，本任务保持未完成，恢复对应前置任务并在其文件范围内做最小修复，不能在验收测试中放宽标准。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo test -p vw-desktop conversation_shell_renders_visual_regression_scenarios_without_panic -- --nocapture
cargo test -p vw-desktop long_session_
cargo test -p vw-desktop streaming_transcript_
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录 before/after 探针输出、四项确定性复杂度计数、两种主题真实渲染结果、聚焦测试与最终完整 clippy 结果；同时确认无关微信渠道改动未被包含。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-7-long-session-acceptance.md`。

### 任务边界

只增加和运行最终验收，不主动扩展生产功能。先阅读 `AGENTS.md`、计划总览、全部前置任务完成记录、任务 1 探针及现有视觉回归测试；确认测试会穿过真实 Iced 文本整形、布局与 draw，而不是只构建控件树。

### 必须遵守

1. 严格遵循测试驱动开发：先写最终复杂度和真实渲染断言；失败时修复对应前置任务实现，不放宽验收。
2. 只修改本任务测试文件；若发现前置实现缺陷，停止本任务，恢复对应前置任务并另行提交修复。
3. 确定性成败依据是操作次数、窗口上界和语义等价；耗时只作同机诊断。
4. 真实渲染覆盖暗黑/明亮主题、中文、动态文本、弹窗附近状态、Markdown、代码、思考和工具块。
5. 中文控件不得使用 `Font::MONOSPACE`；纯 ASCII 代码/路径需要等宽时与中文标签拆开。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试与探针；不得减少 1,000 条历史、128 KiB 流或数百 delta 的覆盖规模。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本计划引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

