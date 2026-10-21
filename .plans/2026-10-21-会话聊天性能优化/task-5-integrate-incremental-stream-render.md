# 任务 5：接入增量流式渲染并移除全文热路径

## 状态

未开始

## 依赖

依赖任务 2、3、4；批次 3。

## 目标

把稳定区/可变尾投影接入真实流式消息链路，使每批 delta 只追加和更新活动尾；移除 UI 主线程的全文哈希/同步全量解析，以及每批后台克隆整条增长消息再全量预热的路径。

## 文件范围

- 修改 `crates/vw-desktop/src/app/state/agent.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/stream/stream_events.rs`
- 修改 `crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/project/helpers.rs`
- 修改 `crates/vw-desktop/src/app/message/project/helpers_tests.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/lifecycle.rs`
- 修改 `crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui.rs`
- 修改 `crates/vw-desktop/src/app/session/chat_ui_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache_tests.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/assistant_body.rs`
- 修改 `crates/vw-desktop/src/app/components/chat_panel/message_view/assistant_body_tests.rs`

## 实现要求

1. 将任务 4 投影放入活动流尾状态，delta 合并后追加到投影并递增显式 revision；生命周期开始、会话切换、取消、失败和完成都必须清理或完成该状态。
2. `finish_active_stream_tail_update` 不再对每一批调用 `app.chat.get(changed_idx).cloned()` 后启动完整 `prepare_stream_message_ui_task`。增长中的正文只更新投影与活动消息局部高度。
3. `message_view` 对流式 assistant 直接借用稳定块与可变尾，不调用 `resolve_visible_text_and_copy_hash` 的全文回退，也不在缓存陈旧时同步调用 `build_render_cache_entry`。
4. 用消息/投影 revision 判断流式视图是否变化；稳定或完成消息继续复用现有完成态缓存，避免把所有缓存系统一次性重写。
5. 工具生命周期等替换型更新必须调用任务 4 的显式局部重建/重置路径，并从最近安全稳定边界恢复；无法安全处理时明确回退一次全量重建并记录原因。
6. 完成、取消或错误终止时只做一次规范化完整解析，生成与现有完成态相同的 render/editor/Markdown 缓存；过期 generation 的异步结果仍须拒绝。
7. 删除或改写当前要求“流式内容变化时重建长消息”的测试，改为断言稳定前缀复用、主线程零全文重建、完成时恰好一次规范化构建。
8. 保留 50/100 ms delta 合并策略作为网络到 UI 的节流层；本任务不靠扩大间隔掩盖复杂度问题。

## 测试先行

1. 先将 `render_cache_streaming_content_change_rebuilds_long_message` 改为期望借用增量投影，确认旧实现失败。
2. 增加 128 KiB、数百次 delta 的事件链路测试，断言流式期间完整解析次数为 0、完成时为 1、稳定前缀解析次数不增长。
3. 增加工具块替换、思考转正文、取消、错误、会话切换和过期 generation 测试。
4. 增加消息视图测试，确认流式 cache 陈旧时不在 view 线程同步构建全量缓存，且中文 Markdown/代码/工具块可见结果正确。
5. 增加最终 transcript 与一次性规范解析的等价断言。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop stream_render_
cargo test -p vw-desktop stream_events_
cargo test -p vw-desktop render_cache_streaming_
cargo test -p vw-desktop message_view_builds_streaming
cargo test -p vw-desktop prepare_stream_message_ui_task
cargo test -p vw-desktop chat_performance_probe -- --ignored --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录流式期间/完成时完整解析次数、消息克隆路径搜索结果、128 KiB 等价测试、探针前后数据、聚焦测试和完整 clippy 结果。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-5-integrate-incremental-stream-render.md`。

### 任务边界

只接入任务 4 的增量投影并删除已确认的流式全文热路径，不重做消息持久化或分块快照。先阅读 `AGENTS.md`、计划总览、任务 2—4 完成记录、所有列出的生产文件和相邻测试，画清 delta、工具替换、generation、完成态缓存和高度更新的实际调用顺序。

### 必须遵守

1. 严格遵循测试驱动开发：先改写旧的全量重建预期并增加事件链路失败测试，再接入最小实现。
2. 只修改本任务文件范围，不扩大流式合并间隔来替代复杂度修复。
3. 流式 view 主线程不得全文哈希或全量解析；稳定前缀不得重复解析；完成态只规范化一次。
4. 工具替换、取消、错误和过期异步结果必须显式处理，不得静默丢内容或扩大回退范围。
5. 保持现有 Markdown、思考块、工具块、复制文本、错误提示和完成态缓存语义。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试和性能探针；不得删除 128 KiB、多 delta 或中文边界用例。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

