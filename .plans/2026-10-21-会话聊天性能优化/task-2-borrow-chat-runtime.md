# 任务 2：消除聊天渲染期运行态克隆

## 状态

未开始

## 依赖

依赖任务 1；批次 2，可与任务 3、4 并行，但不得修改对方文件。

## 目标

让一次聊天面板渲染只借用一次当前 `SessionRuntimeState`，并向消息、思考块和全局状态视图传递最小只读事实，消除每条可见消息重复克隆字符串、队列和编辑器内容的开销。

## 文件范围

- 修改 `crates/vw-desktop/src/app/components/chat_panel/mod.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/think_block.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/global_status.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/think_block_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/global_status_tests.rs`

## 实现要求

1. 在 `chat_panel::view` 入口通过已有 `current_session_runtime_ref()` 取得只读引用，一次性派生消息视图真正需要的 `is_requesting`、模型名、处理状态和时间信息。
2. 调整 `message_view`、`think_block_view` 与全局状态调用签名，让它们接收引用或少量 `Copy`/借用字段；禁止在可见消息循环内调用返回所有权的 `current_session_runtime()`。
3. 不为未来用途复制完整运行态 DTO；只有至少两个实际调用者需要同一组字段时，才定义小型显式渲染上下文。
4. 保持无当前会话、会话切换、请求中、处理计时和模型展示语义不变。
5. 用任务 1 探针比较前后结果；性能重构不得改变可见内容、消息操作或主题样式。

## 测试先行

1. 先扩展聊天壳和消息视图测试：构造含大型 pending 队列、编辑器内容与多条可见消息的运行态，断言渲染结果与现有语义一致。
2. 该优化不改变外部行为，失败信号由编译期调用签名约束和任务 1 工作负载提供；先让测试调用新的借用入口而编译失败，再修改实现。
3. 完成后用 `rg 'current_session_runtime\(\)'` 检查本任务四个生产文件，聊天渲染循环不得保留所有权访问。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop conversation_shell_
cargo test -p vw-desktop message_view_
cargo test -p vw-desktop think_block_
cargo test -p vw-desktop global_status_
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录所有权访问搜索结果、探针前后数据、聚焦测试与完整 clippy 结果；若保留任何运行态克隆，必须在记录中逐处说明不可借用的生命周期原因。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-2-borrow-chat-runtime.md`。

### 任务边界

只消除聊天渲染期 `SessionRuntimeState` 克隆，不改变流式解析、高度索引或分块预热。先阅读 `AGENTS.md`、计划总览、列出的生产文件和相邻测试，确认 `current_session_runtime_ref()` 的生命周期与会话切换语义。

### 必须遵守

1. 严格遵循测试驱动开发：先让测试通过新借用签名产生预期编译失败，再做最小实现直至通过。
2. 只修改本任务文件范围，不顺手重构消息样式或运行态结构。
3. 优先借用与 `Copy` 字段；不得引入完整运行态镜像或无真实调用者的抽象。
4. 保持请求中、处理计时、模型名、思考块和无会话状态的可见行为不变。
5. 测试文件必须独立于业务实现，并覆盖多消息与大型运行态场景。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试和性能探针；不得用缓存预热顺序变化掩盖语义差异。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

