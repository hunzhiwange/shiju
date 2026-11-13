# 会话输入区语音交互验收

## 行为约定

| 输入/阶段 | 入口与结果 |
| --- | --- |
| 空草稿（trim 后为空）、无附件 | 主按钮“开始新的语音聊天”，调用现有 ChatStart，不保证另建文字会话 |
| 中文文字或仅附件 | 主按钮恢复发送，沿用原发送路由 |
| 请求进行中 | 保持原有发送/排队语义 |
| 空闲麦克风 | 独立“听写”，启动现有听写生命周期 |
| 准备 | “正在准备麦克风…”，允许取消，不伪造音量 |
| 录音 | 有界120点历史；左侧取消，右侧停止/完成均发 Finish，悬停不结束 |
| 转写 | “正在转写…”，允许取消，禁止重复结束 |
| 完成 | 回填原草稿，不自动发送；编辑冲突需显式“应用听写”或丢弃 |
| 取消/切换上下文 | 清理在途状态，拒绝迟到结果 |

波形测试使用确定性音量序列，验证历史移位、上限与重复序号衰减；不能据此宣称物理麦克风采样通过。录音条用 Fill 收缩中部，中文采用默认字体。240/640宽度、明暗主题的无截图检查覆盖准备与转写的中文整形；并非真实设备就绪录音波形或窗口可视验收。

## macOS 系统听写

应用麦克风启动 ShiJu 听写，与系统听写独立。系统听写需要将光标放入可编辑文本框，再按系统已配置的听写快捷键；连按两次 Fn/地球仪键仅为示例。设置入口为“系统设置 → 键盘 → 听写”。提示首行是“听写”，说明仅在 macOS 显示。

本次不修改系统设置、不处理用户授权、不模拟粘贴冒充系统听写。待本人使用现有系统配置，分别向会话 Iced 编辑器和普通设置输入框听写中文，确认文字实际进入、无自动发送、会话主按钮恢复发送。长提示在窄窗口的完整可读性及双主题悬停效果尚未获得真实窗口证据。

## 本次验证（2026-11-13）

批次3实现提交 db37c7548；既有8项主测试、11项听写路由测试及Clippy历史通过。批次4复用行为测试，无新增产品行为；将历史保存PNG测试改为无截图的整形检查，遵守 `.plans/AGENTS.md`，不新增窗口或全量构建。

所有命令在一次 `python3 scripts/cargo_slot.py sh -c '…'` 内顺序执行。实际槽2，CARGO_TARGET_DIR、CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，入口退出释放自有锁并保留缓存。

执行命令及结果见下方最终记录。本次没有新增行为修复，未制造无意义红测；批次3历史红测只证明停止提示断言失败，不扩充其证据范围。

- `cargo fmt --all` 正常完成；差异审查只保留目标测试去除截图的变更。
- 默认特性命令均为 `cargo test -p vw-desktop --lib FILTER -- --nocapture`，FILTER依次为 `voice_ui_`、`app::components::input_panel::send_controls_tests`、`app::voice::capture_tests`、`voice_task_07_`。全部退出101，有效测试0；日志分别 `/tmp/voice-ui-4-{main,send,capture,draft}.log`。原因是工作区其它改动新增的 `app/lsp/install.rs:65` 调用 `vw_shared::shell::apply_augmented_path_tokio_command`，默认特性未启用 `shell-tokio`（E0425）；未修改无关代码。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出0（1m18s）；日志 `/tmp/voice-ui-4-clippy.log`。全特性成功不代表默认特性测试通过。
- 替代路径在同槽2第二次入口顺序执行 `cargo test -p vw-desktop --lib --features vw-shared/shell-tokio FILTER -- --nocapture`，使用上述相同四组过滤。

- 替代特性结果：`voice_ui_` 18通过（退出0）；`send_controls_tests` 17通过、4失败（退出101）；`capture_tests` 13通过（退出0）；`voice_task_07_` 32通过（退出0）。日志为 `/tmp/voice-ui-4-feature-{voice_ui_,send_controls_tests,capture_tests,voice_task_07_}.log`。
- 旧发送组失败均为既有底栏结构断言：`bottom_bar_places_workflow_after_full_access_and_before_session_tabs` 寻找已移除的 subagent push；`bottom_bar_keeps_minimal_non_task_utility_and_attachment_clusters`、`bottom_bar_task_mode_moves_attachment_into_utility_and_omits_actions`、`bottom_bar_task_mode_without_pool_keeps_usage_and_attachment_only` 的预期子控件数量较当前多1。与本任务只移除截图保存无关，未扩展修复底栏。
- 差异检查 `git diff --check`（本任务路径）退出0。状态为“已完成，待回归”：保留编号任务文件，待相邻默认特性/旧底栏测试修复后重跑，并由本人完成系统听写输入和提示可读性检查。未宣称真实音频、系统听写或全套旧发送回归通过。

## 统一回归（2026-11-13）

用户本轮明确免除系统听写人工验收，留待后续自行测试；这不作为真实系统听写通过证据。无需再以该项阻塞自动验收收尾。

默认特性重新运行发现LSP安装模块仍调用未启用的 `shell-tokio` helper，首轮 `voice_ui_`、旧发送、采集过滤均编译失败退出101（0测试）。为完成原定默认特性验证，在桌面 crate 对 vw-shared 的现有依赖显式启用 `shell-tokio`，没有新增依赖或改变权限。

同步修正旧发送回归：移除已不存在的子代理按钮源码要求，断言工作流仍位于权限与会话TAB之间；子控件数量及索引对齐当前底栏，继续验证任务模式附件归入工具组、非任务模式附件位于操作组。没有修改底栏产品行为。

首轮后续验证曾被共享工作区定时任务模块的宏歧义和toast类型推断错误中断，退出101；该模块随后由共享工作区更新修复，本任务未修改其实现。历史失败日志保留 `/tmp/voice-ui-4-regression-*.log`。

最终入口为 `python3 scripts/cargo_slot.py sh -c 'cargo fmt --all; …'`，顺序执行 `cargo test -p vw-desktop --lib FILTER -- --nocapture`（默认特性，FILTER同前四组），最后执行 `cargo clippy --all-targets --all-features -- -D warnings`。实际槽3；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，退出释放锁、缓存保留。首轮槽2同理。

最终结果（默认特性）：`voice_ui_` 18通过、`send_controls_tests` 21通过、`capture_tests` 13通过、`voice_task_07_` 32通过，各组退出0、无失败。日志分别为 `/tmp/voice-ui-4-final-voice_ui_.log`、`/tmp/voice-ui-4-fixed-send.log`、`/tmp/voice-ui-4-final-capture_tests.log`、`/tmp/voice-ui-4-final-voice_task_07_.log`。

中间发送组19通过2失败（`/tmp/voice-ui-4-final-send_controls_tests.log`，退出101）揭示任务模式测试多取了一层子树；纠正Tree索引后，最后一次同槽3入口只重跑受影响的发送组及整仓Clippy，避免重复无关验证。`cargo clippy --all-targets --all-features -- -D warnings` 最终退出0（28.08s，`/tmp/voice-ui-4-fixed-clippy.log`）。两轮 `cargo fmt --all` 正常完成，目标路径差异检查退出0。自有槽入口自然结束并释放锁，缓存保留。

自动回归完成，编号任务文件删除；系统听写及真实窗口可读性未实测，按用户本轮“以后我自己测试”的指示留待自行验收，不再作为本轮阻塞。
