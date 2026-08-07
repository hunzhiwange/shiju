# 任务 2: 运行时强制保留内置与 MCP 工具

批次：【批次 2】 依赖批次 1

状态：未开始

目的：会话 `allowed_tools` 过滤时强制保留内置必选工具，并对已注册的 `mcp__*` 动态工具一律保留，避免白名单/UI 过滤导致失效。

来源任务：无

预计会话范围：只改 `helpers` 过滤逻辑与其测试，沿用现有 TodoWrite 强制保留模式并扩展。

## 文件

- 修改：`crates/vw-agent/src/session/processor/helpers.rs`
- 修改：`crates/vw-agent/src/session/processor/helpers_tests.rs`

## 上下文

- 现有 `ALWAYS_RETAINED_TODO_TOOL_IDS` 仅覆盖 TodoRead/TodoWrite 主 ID 与别名。
- 过滤入口：`allowed_tool_ids_for_request_from_specs`；当 `allowed_tools` 非空时与 base 求交后再补回强制项。
- 批次 1 提供 `vw_config_types::tools::{is_builtin_required_tool_id, is_mcp_dynamic_tool_id}`。
- 不要静默扩大权限到未在 base specs 中出现的工具。

## 测试计划

- 行为：请求只允许 `file_read` 时，仍保留 specs 中的内置必选（如 TodoWrite/AskUserQuestion 等）以及 specs 中的 `mcp__*`；非内置非 MCP 且未请求的工具被剔除。
- 失败验证测试：更新/新增 `helpers_tests.rs` 中 `request_filter_always_retains_builtin_required_and_mcp_tools_when_present_in_specs`（可替换过窄的 todo-only 用例或并存并调整旧断言）。
- 失败验证命令：`cargo test -p vw-agent --lib request_filter_always_retains_builtin_required_and_mcp_tools_when_present_in_specs -- --exact`
- 预期失败原因：仍只保留 Todo 工具或未保留 MCP。
- 通过验证命令：同上；并跑 `cargo test -p vw-agent --lib allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set -- --exact` 确认相关旧行为已按新语义更新。
- 模拟策略：用手工 `ToolSpec` 列表，不启动真实会话。

## 步骤

1. 编写/改写失败验证测试，覆盖内置必选与 `mcp__*` 保留、非目标工具剔除。
2. 运行失败验证命令并确认预期失败。
3. 将 `ALWAYS_RETAINED_TODO_TOOL_IDS` 扩展为基于共享目录的强制保留：对 base 中每个工具，若 `is_builtin_required_tool_id` 或 `is_mcp_dynamic_tool_id` 为真则插入 filtered。
4. 同步更新仍假设「仅 Todo 强制保留」的相邻断言。
5. 运行通过验证命令。

## 验证

- 运行：`cargo test -p vw-agent --lib request_filter_always_retains_builtin_required_and_mcp_tools_when_present_in_specs allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set -- --exact`
- 预期：相关测试通过，退出码 0。
- 所需证据：失败验证失败输出；通过验证成功输出；确认未把 base 外工具塞进结果。

## 完成

`allowed_tool_ids_for_request_from_specs` 在非空白名单下保留内置必选与 MCP 动态工具。任务完成后删除本任务文件。
