// 内置必需工具及别名是代理能力协商契约，顺序和集合都不能静默变化。
#[test]
fn builtin_required_tool_catalog_matches_expected_ids_and_aliases() {
    let expected_primary = [
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
    assert_eq!(super::BUILTIN_REQUIRED_TOOL_IDS, expected_primary);

    for id in expected_primary {
        assert!(super::is_builtin_required_tool_id(id), "primary id should match: {id}");
    }

    for (alias, primary) in [
        ("read", "file_read"),
        ("write", "file_write"),
        ("edit", "file_edit"),
        ("shell", "bash"),
        ("question", "AskUserQuestion"),
        ("todoread", "TodoRead"),
        ("todowrite", "TodoWrite"),
    ] {
        assert!(
            super::is_builtin_required_tool_id(alias),
            "alias `{alias}` should match builtin required (primary `{primary}`)"
        );
    }

    assert!(!super::is_builtin_required_tool_id("mcp__rustcodegraph__search"));
    assert!(!super::is_builtin_required_tool_id("web_fetch"));
    assert!(!super::is_builtin_required_tool_id(""));
    assert!(!super::is_builtin_required_tool_id("exec_command"));
}

#[test]
fn optional_tools_default_enabled_without_becoming_required() {
    assert_eq!(super::BUILTIN_REQUIRED_TOOL_IDS.len(), 14);
    for id in [
        "schedule",
        "cron_add",
        "cron_list",
        "cron_remove",
        "cron_update",
        "cron_run",
        "cron_runs",
        "exec_command",
    ] {
        assert!(!super::DEFAULT_ENABLED_TOOL_IDS.contains(&id), "{id}");
    }
    for id in super::BUILTIN_REQUIRED_TOOL_IDS {
        assert!(super::DEFAULT_ENABLED_TOOL_IDS.contains(id), "{id}");
    }
    for id in [
        "pdf_read",
        "http_request",
        "AgentTool",
        "create_thread",
        "memory_store",
        "batch",
        "Browser",
        "Config",
        "image_asset_read",
        "screenshot",
        "Sleep",
        "McpStatus",
    ] {
        assert!(super::DEFAULT_ENABLED_TOOL_IDS.contains(&id), "{id}");
        assert!(!super::is_builtin_required_tool_id(id), "{id}");
    }
}

// MCP 动态工具只按受控前缀识别，普通工具名不能被误归类。
#[test]
fn is_mcp_dynamic_tool_id_detects_prefix() {
    assert!(super::is_mcp_dynamic_tool_id("mcp__rustcodegraph__search"));
    assert!(super::is_mcp_dynamic_tool_id("mcp__github__list_issues"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp__"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp__onlyserver"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp"));
    assert!(!super::is_mcp_dynamic_tool_id("file_read"));
    assert!(!super::is_mcp_dynamic_tool_id(""));
    assert!(!super::is_builtin_required_tool_id("mcp__rustcodegraph__search"));
}
