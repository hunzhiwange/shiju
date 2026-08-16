//! 规范化聊天工具名称展示。
//! 模块集中处理内部工具名到用户可见标签的映射。

pub fn canonical_tool_name(tool_name: &str) -> &str {
    let trimmed = tool_name.trim();
    let normalized = trimmed.to_ascii_lowercase();

    match normalized.as_str() {
        "agent" | "agenttool" | "agent_tool" => "AgentTool",
        "read" | "file_read" | "read_file" => "read",
        "bash" | "shell" | "execute" | "tool_execute" | "terminal" => "bash",
        "brief" => "brief",
        "edit" | "file_edit" | "edit_file" | "editfile" => "file_edit",
        "notebook_edit" | "edit_notebook" | "notebookedit" => "notebook_edit",
        "askuserquestion" | "ask_user_question" | "question" => "question",
        "todowrite" | "todo_write" => "todowrite",
        "todoread" | "todo_read" => "todoread",
        "webfetch" | "web_fetch" => "web_fetch",
        "websearch" | "web_search" | "web_search_tool" => "web_search",
        "browser" => "browser",
        "browseropen" | "browser_open" => "browser_open",
        "lsp" => "lsp",
        "enterplanmode" | "enter_plan_mode" | "plan_enter" => "enter_plan_mode",
        "exitplanmode" | "exit_plan_mode" | "plan_exit" => "exit_plan_mode",
        "verifyplanexecution" | "verify_plan_execution" => "verify_plan_execution",
        "enterworktree" | "enter_worktree" => "enter_worktree",
        "exitworktree" | "exit_worktree" => "exit_worktree",
        "toolsearch" | "tool_search" => "tool_search",
        "workflownode" | "workflow_node" => "workflow_node",
        _ => trimmed,
    }
}

/// 执行 is_known_tool_name 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn is_known_tool_name(tool_name: &str) -> bool {
    let normalized = canonical_tool_name(tool_name).trim().to_ascii_lowercase();

    matches!(
        normalized.as_str(),
        "read"
            | "file_read"
            | "pdf_read"
            | "read_file"
            | "write"
            | "file_write"
            | "file_edit"
            | "notebook_edit"
            | "apply_patch"
            | "glob"
            | "grep"
            | "ls"
            | "list_dir"
            | "bash"
            | "execute"
            | "tool_execute"
            | "terminal"
            | "batch"
            | "todoread"
            | "todowrite"
            | "task"
            | "question"
            | "webfetch"
            | "web_fetch"
            | "web_search"
            | "fetch_webpage"
            | "browser"
            | "browser_open"
            | "open_browser_page"
            | "brief"
            | "lsp"
            | "agent"
            | "agenttool"
            | "codesearch"
            | "glob_search"
            | "content_search"
            | "searchcodebase"
            | "grep_search"
            | "file_search"
            | "semantic_search"
            | "github_repo"
            | "copilot_getnotebooksummary"
            | "vscode_listcodeusages"
            | "skill"
            | "screenshot"
            | "schedule"
            | "cron_add"
            | "cron_list"
            | "cron_run"
            | "cron_update"
            | "cron_remove"
            | "memory_store"
            | "memory_recall"
            | "memory_forget"
            | "git_operations"
            | "git_diff"
            | "get_errors"
            | "get_changed_files"
            | "view_image"
            | "wasm_module"
            | "pushover"
            | "process"
            | "enter_plan_mode"
            | "exit_plan_mode"
            | "verify_plan_execution"
            | "enter_worktree"
            | "exit_worktree"
            | "tool_search"
            | "workflow_node"
    )
}

/// 执行 is_compact_tool_call_trace 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn is_compact_tool_call_trace(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("tool") else {
        return false;
    };
    let Some((name, _)) = rest.trim_start().split_once('(') else {
        return false;
    };
    if name.is_empty() || !name.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
        return false;
    }

    is_known_tool_name(name)
}
