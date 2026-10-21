# 任务 6：限定会话分块预热的快照范围

## 状态

未开始

## 依赖

依赖任务 5；批次 4。

## 目标

让消息编辑器/Markdown 分块预热只捕获请求的 32 条消息，而不是在活动流导致共享缓存失效时克隆完整历史；长会话滚动到未准备区块的成本与请求区块数成正比。

## 文件范围

- 修改 `crates/vw-desktop/src/app/session/active.rs`
- 修改 `crates/vw-desktop/src/app/session/active_tests.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/project/helpers.rs`
- 修改 `crates/vw-desktop/src/app/message/project/helpers_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/input.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/input/input_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/input/shared.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/mod.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/lifecycle.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/open.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/open_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/search/mod.rs`
- 修改 `crates/vw-desktop/src/app/message/view/mod.rs`

## 实现要求

1. 定义明确的分块准备输入，包含原始 `chunk_start_idx`、该块的消息切片快照和 base 标记；后台任务不得再依赖完整 `SharedChatMessages` 才能准备一个块。
2. 在 UI 线程调度点按 `CHAT_UI_CHUNK_SIZE` 精确截取请求块；每块最多复制 32 条消息，越界和空块返回明确空结果。
3. 批量请求多个块时，只复制这些块的并集；重复 chunk start 去重，输出仍携带稳定原始索引。
4. 更新所有 `prepare_session_ui_task`/`prepare_session_ui_chunks_task` 调用者，禁止先调用 `active_shared_chat_messages()` 再由后台切片。
5. `active_shared_chat_messages()` 可继续服务真正需要完整快照的加载/缓存路径，但聊天滚动、打开会话初始预热、权限跳转、搜索跳转和流式附近预热不得使用它捕获整段历史。
6. 保持 prepared phase 的索引、base chunk、generation 丢弃和缓存命中语义不变。

## 测试先行

1. 先增加 1,000 条历史、请求一个尾部块的测试，断言准备输入只持有 32 条且原始索引正确；让现有完整共享快照 API 无法满足新签名而失败。
2. 增加多个相邻/重复/越界 chunk start 测试，断言总捕获条数不超过请求块并集、顺序稳定且无重复。
3. 增加活动流使 `session_chat_cache` 失效的场景，断言滚动预热仍不创建完整历史快照。
4. 覆盖会话打开、权限跳转、搜索跳转和 base chunk 准备结果与改造前一致。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop prepare_session_ui_
cargo test -p vw-desktop active_shared_chat_messages
cargo test -p vw-desktop scroll_changed_
cargo test -p vw-desktop session_ui_prepared_
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录 1,000 条历史下捕获消息数、所有旧完整快照调用点搜索结果、探针前后数据、聚焦测试和完整 clippy 结果。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-6-bounded-chunk-snapshot.md`。

### 任务边界

只限制聊天 UI 分块准备的输入快照，不重构会话持久化或通用共享聊天缓存。先阅读 `AGENTS.md`、计划总览、任务 5 完成记录、`CHAT_UI_CHUNK_SIZE` 定义、所有列出的调用者和相邻测试；用 `rg` 枚举 `prepare_session_ui_task`、`prepare_session_ui_chunks_task` 与 `active_shared_chat_messages()` 的真实调用点。

### 必须遵守

1. 严格遵循测试驱动开发：先用 1,000 条历史断言单块只捕获 32 条并让旧 API 失败，再修改实现与调用者。
2. 只修改本任务文件范围；完整历史缓存仍有真实调用者时不得顺手删除。
3. 分块输入必须保留原始索引、base 标志和 generation 语义；越界必须安全截断。
4. 批量块去重并只复制请求并集，不建立完整历史中间 Vec。
5. 覆盖滚动、会话打开、权限/搜索跳转和流式附近预热调用点。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试和性能探针；不得把 1,000 条 fixture 缩小来规避完整克隆。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

