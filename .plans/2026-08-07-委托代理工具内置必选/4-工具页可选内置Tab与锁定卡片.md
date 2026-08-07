# 任务 4: 工具页可选内置 Tab 与锁定卡片

批次：【批次 3】 依赖批次 1,2

状态：未开始

目的：委托代理工具页用 Tab 分成「可选」「内置」；内置卡片始终选中、不可取消并显示徽章；补齐内置工具中文元数据。

来源任务：无

预计会话范围：聚焦 `tools.rs` 视图与相邻 UI/状态 Tab 字段，依赖任务 3 的合并语义已可用。

## 文件

- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/tools.rs`
- 修改：`crates/vw-desktop/src/app/components/system_settings_agents/tools_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/agents.rs`（增加 tools 子 Tab 状态，例如 `optional`/`builtin`）
- 修改：`crates/vw-desktop/src/app/message/settings/messages.rs`（新增 `ToolsListTabSelected`）
- 修改：`crates/vw-desktop/src/app/message/settings/agents.rs`（处理 Tab 切换消息）
- 可选：`crates/vw-desktop/src/app/components/system_settings_agents/shared.rs`（复用 segment tab 按钮）

## 上下文

- 当前工具页单列表按 `ToolGroup` 分组，卡片均可 `AllowedToolToggled`。
- Tab1「可选」：`available_tools` 中非内置；Tab2「内置」：`available_tools` 中内置必选（若 gateway 暂未返回某内置项，可不伪造卡片，或仅展示 available 交集；不要虚构未注册工具）。
- 内置卡片：视觉选中 +「内置」徽章；不绑定取消 toggle（可无 `on_press` 或按压无操作）。
- 预设/全选/反选控件放在「可选」Tab；「内置」Tab 只读展示。
- Iced：暗黑主题对比足够；滚动条 4px；中文不用 `Font::MONOSPACE`。
- `tool_meta` 需覆盖批次 1 目录中仍显示「未分类」的 ID（如 `AskUserQuestion`、`Config`、`file_edit`、`image_asset_read`、`tool_search`、`verify_plan_execution`、`Browser`/`BrowserOpen`/`WebSearch`/`TodoRead`/`TodoWrite` 等）。

## 测试计划

- 行为：默认「可选」Tab 不含内置 ID；切换到「内置」Tab 只含内置且卡片无取消消息；内置卡片文案含「内置」。
- 失败验证测试：`tools_tests.rs` 新增 `tools_view_splits_optional_and_builtin_tabs`、`builtin_tool_cards_are_locked_and_badged`
- 失败验证命令：`cargo test -p vw-desktop tools_view_splits_optional_and_builtin_tabs builtin_tool_cards_are_locked_and_badged -- --exact`
- 预期失败原因：仍单列表或内置卡片仍发出取消 toggle。
- 通过验证命令：同上；必要时更新既有 `tools_view_*` 树结构断言以适配 Tab。
- 模拟策略：构造 `App`/`DelegateAgentSettingsEntry` 直接 `view` 并检查 widget tree 文本/子节点；不启动 GUI。

## 步骤

1. 编写失败验证测试（含 Tab 切换后树内容断言）。
2. 运行失败验证命令并确认预期失败。
3. 增加子 Tab 状态与消息；按 Tab 过滤列表；内置卡片锁定+徽章；补齐 `tool_meta`；预设行仅在可选 Tab。
4. 运行通过验证命令并修正被布局变更影响的旧 UI 测试。
5. 不做无关主题/设置页重构。

## 验证

- 运行：`cargo test -p vw-desktop tools_view_splits_optional_and_builtin_tabs builtin_tool_cards_are_locked_and_badged -- --exact`
- 预期：通过，退出码 0。
- 所需证据：失败验证失败输出；通过验证成功输出；确认可选/内置分割与锁定行为。
- 代码完成后按仓库协议执行：`cargo clippy --all-targets --all-features -- -D warnings`（若出现与本任务无关的既有失败，交接说明，不顺手大修）。

## 完成

工具页具备「可选 / 内置」Tab；内置工具不可取消并带徽章；MCP 不出现在该列表（依赖任务 3 过滤）。任务完成后删除本任务文件。
