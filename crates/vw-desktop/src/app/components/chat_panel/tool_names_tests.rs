use super::tool_names::{canonical_tool_name, is_compact_tool_call_trace, is_known_tool_name};

#[test]
fn canonical_tool_name_normalizes_common_aliases() {
    assert_eq!(canonical_tool_name(" shell "), "bash");
    assert_eq!(canonical_tool_name("execute"), "bash");
    assert_eq!(canonical_tool_name("Terminal"), "bash");
    assert_eq!(canonical_tool_name("ask_user_question"), "question");
    assert_eq!(canonical_tool_name("plan_enter"), "enter_plan_mode");
}

#[test]
fn canonical_tool_name_covers_every_declared_alias_group() {
    let cases = [
        ("agent_tool", "AgentTool"),
        ("file_read", "read"),
        ("read_file", "read"),
        ("file_edit", "file_edit"),
        ("edit_file", "file_edit"),
        ("editfile", "file_edit"),
        ("edit_notebook", "notebook_edit"),
        ("notebookedit", "notebook_edit"),
        ("askUserQuestion", "question"),
        ("todoWrite", "todowrite"),
        ("todoRead", "todoread"),
        ("webFetch", "web_fetch"),
        ("webSearch", "web_search"),
        ("browserOpen", "browser_open"),
        ("lsp", "lsp"),
        ("exitPlanMode", "exit_plan_mode"),
        ("verifyPlanExecution", "verify_plan_execution"),
        ("exitWorktree", "exit_worktree"),
        ("toolSearch", "tool_search"),
    ];

    for (raw, expected) in cases {
        assert_eq!(canonical_tool_name(raw), expected, "raw={raw}");
    }
}

#[test]
fn known_tool_name_uses_canonical_name() {
    assert!(is_known_tool_name("shell"));
    assert!(is_known_tool_name("execute"));
    assert!(is_known_tool_name("todo_write"));
    assert!(!is_known_tool_name("unknown_tool"));
}

#[test]
fn known_tool_name_accepts_representative_file_runtime_search_and_integration_tools() {
    for name in [
        "pdf_read",
        "write",
        "file_write",
        "apply_patch",
        "glob",
        "grep",
        "ls",
        "list_dir",
        "batch",
        "task",
        "fetch_webpage",
        "open_browser_page",
        "codesearch",
        "glob_search",
        "content_search",
        "searchcodebase",
        "grep_search",
        "file_search",
        "semantic_search",
        "github_repo",
        "copilot_getnotebooksummary",
        "vscode_listcodeusages",
        "skill",
        "screenshot",
        "schedule",
        "cron_add",
        "cron_list",
        "cron_run",
        "cron_update",
        "cron_remove",
        "memory_store",
        "memory_recall",
        "memory_forget",
        "git_operations",
        "git_diff",
        "get_errors",
        "get_changed_files",
        "view_image",
        "wasm_module",
        "process",
    ] {
        assert!(is_known_tool_name(name), "expected known tool {name}");
    }
}

#[test]
fn compact_tool_call_trace_requires_known_function_shape() {
    assert!(is_compact_tool_call_trace("tool bash(command=\"ls\")"));
    assert!(!is_compact_tool_call_trace("tool unknown_tool()"));
    assert!(!is_compact_tool_call_trace("tool bash"));
}

#[test]
fn canonical_tool_name_keeps_unknown_trimmed_name() {
    assert_eq!(canonical_tool_name("  custom_tool  "), "custom_tool");
    assert_eq!(canonical_tool_name("web_search_tool"), "web_search");
    assert_eq!(canonical_tool_name("workflowNode"), "workflow_node");
}

#[test]
fn compact_tool_call_trace_rejects_uppercase_and_missing_prefix() {
    assert!(!is_compact_tool_call_trace("tool Bash(command=\"ls\")"));
    assert!(!is_compact_tool_call_trace("bash(command=\"ls\")"));
    assert!(!is_compact_tool_call_trace("tool bash command=\"ls\")"));
    assert!(!is_compact_tool_call_trace("tool bash-extra(command=\"ls\")"));
    assert!(!is_compact_tool_call_trace("tool (command=\"ls\")"));
}

#[test]
fn known_tool_name_covers_canonicalized_advanced_tools() {
    assert!(is_known_tool_name("ToolSearch"));
    assert!(is_known_tool_name("enterWorktree"));
    assert!(is_known_tool_name("verify_plan_execution"));
}
