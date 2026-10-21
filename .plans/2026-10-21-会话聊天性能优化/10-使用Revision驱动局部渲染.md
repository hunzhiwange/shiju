# 任务 10: 使用 Revision 驱动局部渲染

批次：【批次 6】 依赖批次 5

状态：已完成，待回归验证

目的：以稳定 key 与显式 revision 驱动消息行缓存失效，移除 view 热路径中的完整运行态克隆、完整消息克隆和全文哈希。

来源任务：任务 8 的稳定消息目标与任务 1 的渲染基线

预计会话范围：只改 ChatPanel 到单消息视图的参数传递与完成态缓存键，不接入帧调度或 stable-tail；以未变化行身份与计数测试验证局部失效。

## 文件

- 修改：`crates/vw-desktop/src/app/session/runtime.rs`
- 修改：`crates/vw-desktop/src/app/session/runtime_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/mod.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/render_cache_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/global_status.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/global_status_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/think_block.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/message_view/think_block_tests.rs`

## 上下文

- `current_session_runtime` 当前返回克隆，ChatPanel 还可能按行复制运行态；新增借用访问器，一次取得运行态并向行视图传最小只读上下文。
- 缓存 key 使用 `ChatEntryKey + revision + width bucket + theme/reasoning/display variants`，不能在每次 view 调用对正文全文哈希。
- revision 只在可见内容/结构真实变化时递增；选择态、瞬时 hover 等不影响正文缓存的状态不得滥增 revision。
- 全文哈希可保留在诊断、导入校验或完成态断言中，但不得位于每帧消息 view 热路径。
- 保持暗黑/明亮主题、思考块、工具块和宽度变化的明确失效语义。

## Cargo 共享槽位规则（仅 Rust/Cargo 任务）

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- 行为：渲染 N 条消息后只更新活动条目，N-1 条缓存身份保持，全文哈希计数不增加，运行态只借用一次且没有按行克隆。
- 失败验证测试：`revision_render_reuses_unchanged_rows`、`revision_render_invalidates_changed_entry_only`、`message_view_does_not_hash_full_content`、`conversation_shell_borrows_runtime_once`、`revision_render_invalidates_theme_and_width`。
- 失败验证命令：`cargo test -p vw-desktop revision_render_reuses_unchanged_rows -- --nocapture`
- 预期失败原因：现有缓存依赖内容哈希，view 路径会计算完整 content hash，并通过 `current_session_runtime` 克隆运行态。
- 通过验证命令：`cargo test -p vw-desktop revision_render_ -- --nocapture && cargo test -p vw-desktop message_view_ -- --nocapture && cargo test -p vw-desktop conversation_shell_ -- --nocapture`
- 模拟策略：使用真实消息视图缓存与 App/runtime 数据；仅用任务 1 计数器观测哈希、克隆和构建次数，不 mock 缓存命中。

## 步骤

1. 为稳定 key/revision 缓存命中、单条失效和主题/宽度失效编写失败测试。
2. 为 runtime 借用次数和全文哈希计数编写失败测试并运行。
3. 增加只读借用访问器和最小行上下文，移除按行 `SessionRuntimeState` 克隆。
4. 把缓存键迁移到稳定 key/revision 与明确显示维度，移除 view 热路径全文哈希。
5. 回归 think、工具、状态与主题切换的缓存失效。
6. 运行格式化、聚焦测试和完整 clippy。

## 验证

- 运行：`cargo fmt --all`
- 运行：`cargo test -p vw-desktop revision_render_ -- --nocapture`
- 运行：`cargo test -p vw-desktop message_view_ -- --nocapture`
- 运行：`cargo test -p vw-desktop conversation_shell_ -- --nocapture`
- 运行：`cargo test -p vw-desktop runtime_ -- --nocapture`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：仅改变条目的缓存被重建；未变化行身份保持；view 全文哈希与运行态克隆计数为 0；主题/宽度变化正确失效。
- 所需证据：失败输出、缓存命中/构建/哈希/克隆计数、有效测试数、实际槽位/路径/命令/退出码和 clippy 结果。

## 完成

完成时记录 revision 的递增点、缓存 key 维度和改为借用的 runtime 调用链。不得在本任务接入 delta 调度或改变持久化协议。

## 完成记录

- `PagedChatTimeline::set_active` 在同一稳定活动条目内容变化时递增 revision；持久化条目沿用服务端 revision。
- 完成态缓存键包含 `ChatEntryKey + ChatEntryRevision + 32px width bucket + dark theme + reasoning summary`。
- `ChatPanel::view` 每次构建只借用一次当前 runtime，并向消息行传递 `is_requesting` 与模型引用；思考块沿调用链复用流式标志。
- 槽位：1；`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`。
- 通过：`revision_render_` 3 项、`message_view_` 68 项、`conversation_shell_` 3 项、clippy 全目标全特性。
- 待回归：`runtime_` 过滤命令命中既有 `agents_settings_patch_uses_current_main_model_and_combines_runtime_fields` 失败（`Null != true`），与本任务渲染改动无关；本任务新增 runtime 借用测试单独通过。
