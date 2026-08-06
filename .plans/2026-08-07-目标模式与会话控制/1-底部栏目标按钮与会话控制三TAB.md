# 任务 1: 底部栏目标按钮与会话控制三 TAB

批次：【批次 1】 依赖批次 无

状态：未开始

目的：把目标模式按钮放到完全访问右侧；将「会话控制」拆成代理、工具、技能三个 TAB 入口按钮，仍打开同一会话选择 popover。

来源任务：无

预计会话范围：只改桌面输入栏布局与 popover 打开消息，不改目标模式路由或默认值语义。

## 文件

- 修改：`crates/vw-desktop/src/app/components/input_panel/mod.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/send_controls.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/send_controls_tests.rs`
- 修改：`crates/vw-desktop/src/app/components/input_panel/tests.rs`
- 修改：`crates/vw-desktop/src/app/components/chat_panel/tool_selector.rs`（如需导出 TAB 按钮样式复用）
- 修改：`crates/vw-desktop/src/app/message/view/mod.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/popovers.rs`
- 修改：`crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`
- 修改：`crates/vw-desktop/src/app/message/chat/mod.rs`（如需 `OpenSessionToolSelectorTab(tab)`）
- 修改：`crates/vw-desktop/src/app/message/chat/session.rs`（打开并选中对应 TAB）

## 上下文

- 当前 `bottom_bar`：`permission → workflow → goal → subagent → primary(会话控制) → …`。
- 目标布局：`permission → goal → workflow → subagent → 代理 → 工具 → 技能 → …`。
- `SessionToolSelectorTab::{Agent,Tools,Skills}` 标签已是「代理/工具/技能」。
- 三个按钮共享同一 `show_session_tool_selector_popover`；点击某 TAB：选中该 TAB，若 popover 已对该 TAB 打开则关闭，否则打开；暗黑对比与 4px 滚动条保持不变。
- 禁止对中文使用 `Font::MONOSPACE`。

## 测试计划

- 行为：底部栏构造含三个会话 TAB 入口；打开消息会设置对应 `active_tab` 并显示 popover；再点同一 TAB 关闭。
- 失败验证测试：`bottom_bar_places_goal_after_full_access_and_session_tabs`、`open_session_tool_selector_tab_selects_and_toggles_popover`
- 失败验证命令：`cargo test -p vw-desktop --lib bottom_bar_places_goal_after_full_access -- --nocapture` 与 `cargo test -p vw-desktop --lib open_session_tool_selector_tab -- --nocapture`
- 预期失败原因：尚无三 TAB 入口或打开消息路径。
- 通过验证命令：同上。
- 模拟策略：构造 Element / 驱动 message update，不测像素。

## 步骤

1. 编写布局与 popover TAB 打开的失败验证测试。
2. 运行失败验证命令并确认预期失败。
3. 实现三 TAB 按钮、消息与 `bottom_bar` 顺序调整；移除单一「会话控制」主按钮（或降级为内部复用）。
4. 运行通过验证命令与 `cargo clippy --all-targets --all-features -- -D warnings`。
5. 仅在测试通过后微调 tooltip/高亮。

## 验证

- 运行：上述聚焦测试与 clippy。
- 预期：测试通过；clippy 无本次新增警告。
- 所需证据：失败/通过测试输出；确认不再只有单一「会话控制」入口。

## 完成

完成后输入栏目标按钮在完全访问右侧，其右为工作流/子智能体与代理/工具/技能三个 TAB 入口。不要改目标模式默认值或流路由。
