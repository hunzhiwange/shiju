/// Agent 基础闭环必需的主工具；权限策略仍可拒绝具体操作。
pub const BUILTIN_REQUIRED_TOOL_IDS: &[&str] = &[
    "file_read",
    "file_write",
    "file_edit",
    "ls",
    "glob",
    "grep",
    "apply_patch",
    "bash",
    "bash_wait",
    "bash_cancel",
    "skill",
    "AskUserQuestion",
    "TodoRead",
    "TodoWrite",
];

/// 新智能体的初始工具选择；定时任务默认关闭，已保存的可选选择不得自动补回。
pub const DEFAULT_ENABLED_TOOL_IDS: &[&str] = &[
    "file_read",
    "file_write",
    "ls",
    "glob",
    "grep",
    "apply_patch",
    "bash",
    "bash_wait",
    "bash_cancel",
    "pdf_read",
    "http_request",
    "screenshot",
    "AgentTool",
    "create_thread",
    "fork_thread",
    "list_threads",
    "read_thread",
    "send_message_to_thread",
    "handoff_thread",
    "set_thread_title",
    "set_thread_pinned",
    "set_thread_archived",
    "wait_threads",
    "skill",
    "memory_forget",
    "memory_recall",
    "memory_store",
    "batch",
    "AskUserQuestion",
    "Browser",
    "BrowserOpen",
    "Config",
    "TodoRead",
    "TodoWrite",
    "WebSearch",
    "file_edit",
    "image_asset_read",
    "Sleep",
    "McpStatus",
    "browser",
    "browser_open",
    "web_search",
    "websearch",
    "web_search_tool",
    "edit",
    "sleep",
    "mcp_status",
];

/// 同一必需能力的兼容入口，不计为新增主工具。
const BUILTIN_REQUIRED_TOOL_ALIASES: &[&str] =
    &["read", "write", "edit", "shell", "question", "todoread", "todowrite"];

/// 判断工具是否为必需能力或其兼容入口。
pub fn is_builtin_required_tool_id(tool_id: &str) -> bool {
    BUILTIN_REQUIRED_TOOL_IDS.contains(&tool_id) || BUILTIN_REQUIRED_TOOL_ALIASES.contains(&tool_id)
}

/// 判断工具 ID 是否为 MCP 动态工具（形如 `mcp__{server}__{tool}`）。
///
/// 要求以 `mcp__` 为前缀，且 `__` 分隔后至少两段非空（server 与 tool）。
pub fn is_mcp_dynamic_tool_id(tool_id: &str) -> bool {
    let Some(rest) = tool_id.strip_prefix("mcp__") else {
        return false;
    };
    let mut parts = rest.split("__");
    match (parts.next(), parts.next()) {
        (Some(server), Some(tool)) => {
            !server.is_empty() && !tool.is_empty() && parts.all(|part| !part.is_empty())
        }
        _ => false,
    }
}
