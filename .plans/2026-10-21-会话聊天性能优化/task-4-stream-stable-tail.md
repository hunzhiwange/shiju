# 任务 4：建立流式稳定区与可变尾模型

## 状态

未开始

## 依赖

依赖任务 1；批次 2，可与任务 2、3 并行，但不得修改对方文件。

## 目标

建立与 UI 框架无关、可独立验证的流式渲染投影：已确认的 Markdown/自定义块进入只解析一次的稳定区，最后一个未闭合结构留在可变尾；完成时生成与现有全量解析一致的规范结果。

## 文件范围

- 修改 `crates/vw-desktop/src/app/session.rs`
- 新增 `crates/vw-desktop/src/app/session/stream_render.rs`
- 新增 `crates/vw-desktop/src/app/session/stream_render_tests.rs`

## 实现要求

1. 定义单一职责的流式投影状态，至少保存源文本长度、稳定源边界、稳定渲染块、可变尾、单调递增 revision 和是否需要规范化重建。
2. 追加型 assistant delta 只扫描“上次未稳定尾部 + 新 delta”，已经稳定的源文本和渲染块不得重复解析或复制。
3. 只有结构安全边界才能提交稳定区：UTF-8 字符边界、换行、代码围栏、自定义 `<think>`/工具块闭合状态都必须纳入判断；未闭合内容保留在尾部。
4. 对非追加替换提供显式拒绝或从最近稳定边界重建的路径，不得静默将替换当作追加。
5. `finish` 必须调用现有规范化解析入口校验最终结果，并只发生一次完整构建；中途异常重置要返回明确原因。
6. 不在本任务接入 `ChatPanel` 或事件循环，不改变现有消息状态；该模块应是纯状态转换，便于确定性测试。

## 测试先行

1. 先增加逐字符、逐 token、逐行追加测试，断言最终可见块与一次性 `build_render_cache_entry` 一致；确认模块尚未实现时测试失败。
2. 增加稳定前缀工作量测试：追加数百个 delta 后，每个已稳定片段只解析一次；使用确定性解析计数，不使用耗时阈值。
3. 覆盖中文多字节边界、未闭合代码围栏、跨 delta 的 `<think>`、工具块和普通 Markdown。
4. 覆盖非追加替换、清空、完成和重复完成，要求明确结果或错误，不允许隐式数据丢失。

## 验证

按顺序执行：

```bash
cargo fmt --all
cargo test -p vw-desktop stream_render_
cargo test -p vw-desktop render_cache_
cargo clippy --all-targets --all-features -- -D warnings
```

## 完成证据

完成时记录逐 delta 与一次性解析等价结果、稳定前缀解析计数、边界场景覆盖、聚焦测试和完整 clippy 结果。

## 完整编码提示词

你正在执行 `.plans/2026-10-21-会话聊天性能优化/task-4-stream-stable-tail.md`。

### 任务边界

只实现纯流式稳定区/可变尾模型，不接入现有事件处理或 ChatPanel。先阅读 `AGENTS.md`、计划总览、`message_view/parse.rs`、`message_view/render_cache.rs`、现有自定义块测试，以及 Codex 对照路径 `codex-rs/tui/src/streaming/controller.rs`、`render.rs`；迁移工作量原则，不复制 Ratatui 专用终端逻辑。

### 必须遵守

1. 严格遵循测试驱动开发：先写逐增量等价、稳定前缀不重算和结构边界失败测试，再做最小实现。
2. 只修改本任务文件范围，不接线 UI、消息事件或高度索引。
3. 复用现有解析类型和规则，不实现第二套不兼容 Markdown/工具协议。
4. 对非追加内容显式返回替换/重建结果；不做伪增量支持。
5. 测试必须覆盖中文 UTF-8、代码围栏、思考块、工具块、完成与重复完成。
6. 使用 `cargo fmt --all` 格式化；不要运行 `cargo fmt --all -- --check`、裸 `cargo test`、wasm 全量检查或 Makefile 命令。
7. 执行列出的聚焦测试；不得通过减少 delta 数量或移除结构边界用例规避失败。
8. 执行 `cargo clippy --all-targets --all-features -- -D warnings`；只修复本任务引入的问题，无关既有失败写入完成记录。
9. 所有 Cargo/Rust 编译命令共享全局唯一 1 个编译槽；运行前确认槽空闲，运行期间不得与其他任务的 Cargo、rustc、rustup、rust-analyzer 或编译命令重叠，完成后立即释放。
10. 完成后把状态改为“已完成”，填写验证证据，并调用 `$git-commit` skill 单独提交本任务。

