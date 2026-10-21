# 任务 3：消除同长度高度索引重建

## 状态

未开始

## 依赖

依赖任务 1；批次 2，可与任务 2、4 并行，但不得修改对方文件。

## 目标

让活动流式消息的高度更新只修改对应 Fenwick 节点；消息数量不变时，`ChatVirtualizer::sync_len` 不再复制高度数组或重建完整高度索引。

## 文件范围

- 修改 `crates/vw-desktop/src/app/components/chat_panel/height_index.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/virtualizer.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/height_index_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/virtualizer_tests.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui_tests.rs`

## 实现要求

1. 为测试增加无生产开销的索引重建观测手段，先证明当前同长度 `sync_len` 会调用完整 `set_heights`。
2. `sync_len` 在长度未变化时不得重建索引；仍须保留 runway、可视范围和滚动锚点所需的一致性更新。
3. 长度只增加少量消息时增量追加估算高度；截断或会话整体替换时允许明确的完整重建。
4. `sync_chat_message_estimated_heights_len` 不得在长度未变化时通过 `to_vec()` 再调用全量 `set_estimated_heights`。
5. `apply_stream_tail_estimated_height` 继续只更新活动消息节点，并正确返回是否发生有效高度变化。
6. 保持空聊天、追加、截断、切换会话、滚动锚点与 overscan 语义。

## 测试先行

1. 先增加 `chat_virtualizer_same_len_sync_does_not_rebuild_height_index`，使用测试观测值断言重建次数不增加；确认它在现有实现上失败。
2. 增加 10,000 条高度的场景，重复更新尾部 1,000 次，断言只有点更新、前缀高度和可视范围保持不变；不使用耗时阈值。
3. 增加追加、截断和会话替换测试，证明需要重建的路径仍正确。
4. 增加 chat UI 测试，证明流式尾高度刷新在消息数量不变时不会触发完整估算高度同步。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop height_index_
cargo test -p vw-desktop chat_virtualizer_
cargo test -p vw-desktop stream_tail_estimated_height
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录失败测试修复前后的重建次数、10,000 条消息点更新断言、探针数据、聚焦测试和完整 clippy 结果。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-3-incremental-height-index.md`。

### 任务边界

只优化高度索引同步和活动尾高度更新，不修改流式正文解析或聊天消息缓存。先阅读 `AGENTS.md`、计划总览、`height_index.rs`、`virtualizer.rs`、`session/chat_ui.rs` 及相邻测试，确认 Fenwick 更新、滚动锚点和 runway 约束。

### 必须遵守

1. 严格遵循测试驱动开发：先写可确定失败的同长度不重建测试，再做最小实现。
2. 只修改本任务文件范围，不改变 overscan 常量或滚动交互。
3. 测试工作量使用重建/点更新计数，不以固定毫秒数作为正确性标准。
4. 同长度走常量路径；增长走增量追加；只有截断、整体替换等明确路径可以完整重建。
5. 保持空列表、会话切换、滚动锚点和估算高度回写语义。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试和性能探针；不得删除大规模确定性用例。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

