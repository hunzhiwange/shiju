//! 请求工具集合交集、别名与动态工具授权边界。

use super::{allowed_tool_ids_for_request_from_specs, allowed_tool_ids_from_specs};
use serde_json::json;
use std::collections::HashSet;
use tool_types as tools;

#[test]
fn request_filter_keeps_required_tools_and_excludes_unselected_optional_tools() {
    let specs = vec![
        tools::ToolSpec::new(
            tools::todo_ids::TODO_WRITE_TOOL_ID,
            "Write todos",
            json!({"type": "object"}),
        )
        .with_aliases([tools::todo_ids::TODO_WRITE_TOOL_ALIAS]),
        tools::ToolSpec::new(
            "AskUserQuestion",
            "Ask the user a question",
            json!({"type": "object"}),
        )
        .with_aliases(["question"]),
        tools::ToolSpec::new(
            "mcp__rustcodegraph__search",
            "Search code graph",
            json!({"type": "object"}),
        )
        .with_aliases(["mcp_alias"]),
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "web_fetch",
            "Fetch URL",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "skill",
            "Read skill instructions",
            json!({"type": "object"}),
        ),
    ];

    let allowed =
        allowed_tool_ids_for_request_from_specs(&specs, &json!({"allowed_tools": ["file_read"]}));

    assert_eq!(
        allowed,
        HashSet::from([
            "file_read".to_string(),
            "skill".to_string(),
            tools::todo_ids::TODO_WRITE_TOOL_ID.to_string(),
            tools::todo_ids::TODO_WRITE_TOOL_ALIAS.to_string(),
            "AskUserQuestion".to_string(),
            "question".to_string(),
        ])
    );
    assert!(!allowed.contains("web_fetch"));
    assert!(allowed.contains(tools::todo_ids::TODO_WRITE_TOOL_ID));
    assert!(allowed.contains("AskUserQuestion"));
    assert!(!allowed.contains("mcp__rustcodegraph__search"));
    assert!(!allowed.contains("mcp_alias"));
    assert!(!allowed.contains("missing-tool"));
}

#[test]
fn allowed_tool_ids_for_request_does_not_expand_to_unselected_dynamic_mcp_specs() {
    let specs = vec![
        tools::ToolSpec::new(
            "mcp__rustcodegraph__search",
            "Search code graph",
            json!({"type": "object"}),
        )
        .with_aliases(["mcp_alias"]),
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        ),
    ];

    let allowed = allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"allowed_tools": ["file_read", "missing"]}),
    );

    assert_eq!(allowed, HashSet::from(["file_read".to_string()]));
    assert!(allowed_tool_ids_from_specs(&specs).contains("mcp_alias"));
}

#[test]
fn allowed_tool_ids_from_specs_collects_ids_aliases_and_deduplicates_collisions() {
    let specs = vec![
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        )
        .with_aliases(["read", "shared"]),
        tools::ToolSpec::new(
            "file_write",
            "Write file",
            json!({"type": "object"}),
        )
        .with_aliases(["write", "shared", "file_read"]),
    ];

    assert_eq!(
        allowed_tool_ids_from_specs(&specs),
        HashSet::from([
            "file_read".to_string(),
            "read".to_string(),
            "shared".to_string(),
            "file_write".to_string(),
            "write".to_string(),
        ])
    );
    assert!(allowed_tool_ids_from_specs(&[]).is_empty());
}

#[test]
// 缺少工具列表表示沿用默认集合，显式空列表表示禁用全部工具。
fn request_filter_distinguishes_missing_list_from_explicit_empty_list() {
    let specs = vec![
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "custom_tool",
            "Custom tool",
            json!({"type": "object"}),
        ),
    ];
    let all = HashSet::from(["file_read".to_string(), "custom_tool".to_string()]);

    for options in
        [json!({}), json!({"allowed_tools": null}), json!({"allowed_tools": "file_read"})]
    {
        assert_eq!(allowed_tool_ids_for_request_from_specs(&specs, &options), all);
    }
    for options in [json!({"allowed_tools": []}), json!({"allowed_tools": [null, false, 42, "  "]})]
    {
        assert_eq!(
            allowed_tool_ids_for_request_from_specs(&specs, &options),
            HashSet::from(["file_read".to_string()])
        );
    }
}

#[test]
fn request_filter_returns_empty_when_tools_are_explicitly_disabled() {
    let specs = vec![tools::ToolSpec::new(
        "AskUserQuestion",
        "Ask the user a question",
        json!({"type": "object"}),
    )];

    assert!(
        allowed_tool_ids_for_request_from_specs(
            &specs,
            &json!({"disable_tools": true, "allowed_tools": ["AskUserQuestion"]}),
        )
        .is_empty()
    );
}

#[test]
fn request_filter_trims_names_accepts_aliases_and_discards_unknown_tools() {
    let specs = vec![
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        )
        .with_aliases(["read"]),
        tools::ToolSpec::new(
            "file_write",
            "Write file",
            json!({"type": "object"}),
        ),
    ];

    let allowed = allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"allowed_tools": [" read ", "file_write", "missing", "read"]}),
    );

    assert_eq!(
        allowed,
        HashSet::from(["file_read".to_string(), "read".to_string(), "file_write".to_string()])
    );
}

#[test]
fn request_tool_list_keeps_registered_builtins_without_expanding_custom_tools() {
    let specs = vec![
        tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "memory_store",
            "Store memory",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "custom_tool",
            "Custom tool",
            json!({"type": "object"}),
        ),
        tools::ToolSpec::new(
            "other_custom_tool",
            "Other custom tool",
            json!({"type": "object"}),
        ),
    ];
    let options = json!({"allowed_tools": ["custom_tool"]});

    let allowed = allowed_tool_ids_for_request_from_specs(&specs, &options);
    assert_eq!(allowed, HashSet::from(["file_read".to_string(), "custom_tool".to_string()]));
}

#[test]
fn request_filter_keeps_shell_lifecycle_but_does_not_restore_optional_tools() {
    let ids =
        ["bash", "bash_wait", "bash_cancel", "pdf_read", "memory_store", "Browser", "AgentTool"];
    let specs = ids
        .iter()
        .map(|id| tools::ToolSpec::new(*id, "test", json!({"type":"object"})))
        .collect::<Vec<_>>();
    let allowed = allowed_tool_ids_for_request_from_specs(&specs, &json!({"allowed_tools":[]}));
    assert_eq!(
        allowed,
        ["bash", "bash_wait", "bash_cancel"].into_iter().map(String::from).collect()
    );
    let selected =
        allowed_tool_ids_for_request_from_specs(&specs, &json!({"allowed_tools":["pdf_read"]}));
    assert!(selected.contains("pdf_read"));
    assert!(!selected.contains("memory_store"));
}
