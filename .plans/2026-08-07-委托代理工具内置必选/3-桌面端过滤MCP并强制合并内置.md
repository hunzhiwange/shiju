# 任务 3: 桌面端过滤 MCP 并强制合并内置工具

批次：【批次 2】 依赖批次 1

状态：未开始

目的：委托代理 `available_tools` 去掉 MCP 动态工具；切换/预设/全选/反选路径强制保留内置必选，且无法取消内置。

来源任务：无

预计会话范围：改 agents 消息与状态合并逻辑及测试，不改工具页渲染布局。

## 文件

- 修改：`crates/vw-desktop/src/app/message/settings/agents.rs`
- 修改：`crates/vw-desktop/src/app/message/settings/agents_tests.rs`
- 修改：`crates/vw-desktop/src/app/state/settings/agents.rs`（如把 merge/filter 辅助放到状态层更清晰）
- 可选修改：`crates/vw-desktop/src/app/config_gateway.rs` / `config_gateway_tests.rs`（仅当选择在加载规范化阶段统一过滤 MCP）

## 上下文

- `available_tools` 来自 gateway `/v1/tools`，可含 `mcp__*`；同字段也被员工/自主/会话工具选择复用，过滤 MCP 后它们也不应再出现。
- `AllowedToolToggled` / `SelectAll` / `InvertSelection` / `ApplyPreset` 当前可去掉任意工具。
- 批次 1 提供 `is_builtin_required_tool_id` / `is_mcp_dynamic_tool_id`。
- 合并规则：最终 `allowed_tools = (用户可选选择) ∪ (available_tools ∩ 内置必选)`；对内置的 `enabled=false` 切换直接忽略。

## 测试计划

- 行为：加载含 `mcp__rustcodegraph__search` 的列表后 `available_tools` 不含该项；关闭内置工具的 toggle 不生效；反选/预设后内置仍在 `allowed_tools`。
- 失败验证测试：`agents_tests.rs` 新增/改写 `available_tools_exclude_mcp_dynamic_ids`、`allowed_tool_messages_keep_builtin_required_tools`
- 失败验证命令：`cargo test -p vw-desktop available_tools_exclude_mcp_dynamic_ids allowed_tool_messages_keep_builtin_required_tools -- --exact`
- 预期失败原因：仍保留 MCP，或 toggle/invert/preset 可去掉内置。
- 通过验证命令：同上命令退出码 0。
- 模拟策略：直接构造 `App`/`agents_settings` 状态并发消息，不连真实 gateway。

## 步骤

1. 编写失败验证测试。
2. 运行失败验证命令并确认预期失败。
3. 在加载 `available_tools` 的路径过滤 `is_mcp_dynamic_tool_id`；抽出 `merge_builtin_required_tools(allowed, available)`；在 toggle(关)、invert、preset、select-all、默认填充路径调用。
4. 运行通过验证命令，并检查邻近 `allowed_tool_messages_sort_select_invert_and_apply_presets` 是否需同步断言。
5. 不做 UI Tab 改动（留给任务 4）。

## 验证

- 运行：`cargo test -p vw-desktop available_tools_exclude_mcp_dynamic_ids allowed_tool_messages_keep_builtin_required_tools -- --exact`
- 预期：通过，退出码 0。
- 所需证据：失败验证失败输出；通过验证成功输出；确认 MCP 不在 `available_tools`，内置无法被消息路径移除。

## 完成

桌面配置路径不再把 MCP 动态工具列入可选集合，且内置必选始终留在 `allowed_tools`。任务完成后删除本任务文件。
