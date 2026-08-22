# TUI v3 Codex 界面复刻计划

目标：在不改变 VibeWindow API 网关、会话和安全语义的前提下，新建 `tui_v3` 并一比一复刻 Codex TUI 的视觉语言与主要交互表面。

范围：新增可插拔视觉宿主和 `tui_v3`；复刻无边框主界面、消息历史、工具分组、输入框、状态行、欢迎页、审批问答、功能选择面板、全文记录与滚动反馈；将 CLI 默认交互入口切到 v3；保留现有 gateway/state/reducer/command 管线。

范围外：不移植 Codex app-server、协议、agent/core、配置和业务命令；不复制 Codex 自定义终端滚屏实现；不修改网关 API；不删除 `tui_v2`；不增加运行时主题开关或兼容迁移层。

假设：`tui_v3` 当前不存在；`tui_v2` 已通过 `GatewayUiRuntime → UiRuntimeEvent → TuiState` 隔离业务与渲染；Ratatui 0.30/Crossterm 0.29 足以复刻目标视觉；“一比一”指布局、间距、符号、颜色层级和表面交互一致，产品名与 VibeWindow 专属能力保持自身语义。

## 设计决策

- 方案一“直接复制 Codex TUI”会带入 app-server 和大量协议耦合；方案二“重写全部 TUI 业务层”会复制网关逻辑；选择方案三“共享现有业务宿主、插入新视觉 surface”。
- v3 初期允许引用 `tui_v2` 中已稳定的 model/state/runtime/controller；不为命名整洁进行大规模搬迁。视觉稳定后再单独评估共享模块改名。
- 使用终端默认前景/背景与 ANSI cyan/green/red/magenta，去掉现有大面积 RGB 背景、面板边框和胶囊堆叠；暗色、亮色和无色终端由默认色自然适配。
- 主循环继续使用现有 alternate-screen/虚拟列表实现；Codex 的 inline scrollback、自定义双缓冲 Terminal 和 app-server 会话层仅作为行为参考，不纳入本次移植。

## 架构说明

- `tui_v2::app::TuiApp` 保持 gateway/session/stream/command 所有权，通过窄 `TuiSurface` 接口调用 v2 或 v3 renderer。
- `tui_v3` 只读取 `TuiState`/selectors 并返回现有布局、测量和滚动反馈，不直接持有 `GatewayClient`，不解析 gateway JSON，不决定审批结果。
- 所有外部文本继续走 `sanitize_terminal_text`；所有 `UiOverlay` 分支显式渲染，安全敏感界面不得因复刻而默认批准或隐藏拒绝选项。
- 单元测试独立文件；视觉变化使用 TestBackend 全 cell golden fixture，覆盖符号、样式、光标、宽字符和窄终端。

## Codex 参考源

| 视觉面 | 主要参考文件 | 参考快照 |
| --- | --- | --- |
| 全局色彩与层级 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/styles.md` | 无 |
| 主帧与输入/服务事件协作 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/app.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/app/startup.rs` | `tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__chatwidget_exec_and_status_layout_vt100_snapshot.snap` |
| Renderable 组合 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/render/renderable.rs` | `tui/src/render/snapshots/` |
| 用户/助手/推理消息 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/history_cell/messages.rs` | `tui/src/history_cell/snapshots/` |
| 工具、命令和探索分组 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/history_cell/exec.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/exec_cell/render.rs` | `tui/src/chatwidget/tests/exec_flow.rs`、`tui/src/history_cell/snapshots/` |
| Markdown 与代码高亮 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/markdown_render.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/render/highlight.rs` | `tui/src/history_cell/tests.rs` |
| 输入框与动态页脚 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/chat_composer.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/footer.rs` | `tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__chat_composer__tests__empty.snap`、`...footer__tests__footer_shortcuts_default.snap` |
| 运行状态 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/status_indicator_widget.rs` | `tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__status_widget_active.snap` |
| 审批与用户问答 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/approval_overlay.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/request_user_input/` | `...approval_overlay__tests__network_exec_prompt.snap`、`...request_user_input__tests__request_user_input_options.snap` |
| 通用选择面板 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/list_selection_view.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/bottom_pane/selection_popup_common.rs` | `tui/src/bottom_pane/snapshots/*list_selection*` |
| 欢迎与会话头 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/onboarding/welcome.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/history_cell/session.rs` | `tui/src/history_cell/snapshots/*session_header*` |
| 全文记录覆盖层 | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/pager_overlay.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/pager_overlay/scrolling.rs` | `tui/src/pager_overlay/scrolling_tests.rs` |
| 终端行为（仅参考） | `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/tui.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/custom_terminal.rs`、`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui/src/insert_history.rs` | 不移植 |

## 验收口径

- 主界面不存在整屏彩色面板、粗边框、固定标题栏或 pill 堆叠；内容以空行、缩进、`›`、`•`、`└` 建立层次。
- 相同状态在 40、60、80、100、120 列下不截断双宽字符、不越界、不覆盖光标；窄屏按 Codex 优先级收缩页脚信息。
- 用户、助手、thinking、step、tool、tool result、progress、system、error、attachment、compact boundary、unknown 均有确定渲染。
- 空闲、运行、排队、历史搜索、命令补全、审批、问答、错误、全文记录和 VibeWindow 专属面板均有 golden fixture。
- `GatewayChatStreamRequest`、`UiRuntimeEvent` 适配、session restore、permission reply 和 slash command 行为不因 v3 改变。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实 Ratatui buffer、现有 reducer/selectors 和真实 request 构造；仅在网络/终端不可确定边界使用现有 fake。
- 每个代码任务结束执行聚焦测试与 `cargo clippy --all-targets --all-features -- -D warnings`；不运行仓库禁止的全量测试、fmt check、wasm check 或 Makefile 命令。
