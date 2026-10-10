# 命令行输入响应回归证据

日期：2026-10-10

结论：本计划的自动化聚焦场景、强制静态检查和真实长会话流式 TTY 场景全部通过。

## RustCodeGraph

- `rustcodegraph status`：4293 个文件、70761 个节点、236064 条边。
- `rustcodegraph sync`：退出码 0，0 个文件需要同步。
- `git diff --name-only -- crates/vw-cli crates/vw-tui | rustcodegraph affected --stdin --quiet`：退出码 0，返回：
  - `crates/vw-tui/src/chatwidget/tests/exec_flow.rs`
  - `crates/vw-tui/src/chatwidget/tests/status_and_layout.rs`

## 聚焦测试

- 计划原命令 `cargo test -p vw-cli cli::tui::runner_tests -- --nocapture`：退出码 0，但模块过滤器命中 0 个测试，不作为通过证据。
- 实际模块路径 `cargo test -p vw-cli cli::runtime::tui::runner_tests -- --nocapture`：16 passed、0 failed，退出码 0。Gateway fixture 已与当前 workspace `directory` 查询契约对齐。
- `active_runner_drain_yields_after_stream_budget`：1 passed、0 failed，退出码 0。
- `ready_key_burst_is_applied_before_next_frame`：1 passed、0 failed，退出码 0。
- `cargo test -p vw-tui --lib chatwidget::rendering::tests -- --nocapture`：6 passed、0 failed，退出码 0。快照测试将动态 spinner 规范化为首帧，不修改或接受新的视觉结果。
- `unchanged_long_transcript_reuses_layout_and_projects_only_viewport`：1 passed、0 failed，退出码 0。
- `chatwidget_transcript_scrollbar_and_navigation_are_wired`：1 passed、0 failed，退出码 0。
- `cargo test -p vw-tui --lib chatwidget::transcript::tests -- --nocapture`：6 passed、0 failed，退出码 0。

## 静态检查

- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0。

## 手工 TTY 回归

已在宿主 PTY 中运行 `target-dir/debug/shiju agent`，使用本地 Gateway `127.0.0.1:42617`，并通过 `/resume` 恢复已有会话。

- 恢复后可见多屏历史记录，会话与 transcript 正常渲染。
- 提交“请连续输出一篇至少两千字的中文科幻故事”，Gateway 持续流式输出超过 40 秒。
- 流式输出期间一次写入 120+ 个 ASCII/中文混合字符；草稿按原顺序连续显示为三行，未观察到丢失、乱序或成批追赶。
- 向 PTY 发送 PageUp/PageDown 键序列，长 transcript 持续正常渲染，无卡死或 panic。
- 通过 bracketed-paste 发送 `粘贴验证Paste123`，文本按顺序追加到已有草稿尾部。
- 原长文请求提交成功，流事件持续到达；最后使用 Esc 退出，终端的 alternate screen、mouse capture 和 bracketed paste 模式均正常恢复。

## 差异审查

- 本计划变更范围为 `vw-cli` TUI 调度和 `vw-tui` transcript/rendering。
- 未新增用户配置、功能依赖或 Gateway 协议变更。
- 未接受无关 spinner 快照变更，未修改桌面 UI。

## 最终组合回归

`cargo test -p vw-cli cli::runtime::tui::runner_tests -- --nocapture && cargo test -p vw-tui --lib chatwidget::rendering::tests -- --nocapture && cargo test -p vw-tui --lib chatwidget::transcript::tests -- --nocapture && cargo clippy --all-targets --all-features -- -D warnings`

结果：退出码 0；runner 16 passed、0 failed，rendering 6 passed、0 failed，transcript 6 passed、0 failed，clippy 无 warning/error。
