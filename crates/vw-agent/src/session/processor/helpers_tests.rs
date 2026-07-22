use super::*;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set() {
    let base = allowed_tool_ids(None);
    assert!(!base.is_empty());

    let selected = base.iter().next().expect("at least one registered tool").clone();
    let filtered = allowed_tool_ids_for_request(
        None,
        &json!({"allowed_tools": [selected.clone(), "missing-tool", "  ", 42]}),
    );
    assert_eq!(filtered, HashSet::from([selected]));

    assert_eq!(allowed_tool_ids_for_request(None, &json!({})), base);
    assert_eq!(allowed_tool_ids_for_request(None, &json!({"allowed_tools": ["  ", 42]})), base);
}

#[test]
fn allowed_tool_ids_for_request_uses_dynamic_specs() {
    let specs = vec![
        crate::app::agent::tools::ToolSpec::new(
            "mcp__rustcodegraph__search",
            "Search code graph",
            json!({"type": "object"}),
        )
        .with_aliases(["mcp_alias"]),
        crate::app::agent::tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        ),
    ];

    let allowed = allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"allowed_tools": ["mcp__rustcodegraph__search", "missing"]}),
    );

    assert_eq!(allowed, HashSet::from(["mcp__rustcodegraph__search".to_string()]));
    assert!(allowed_tool_ids_from_specs(&specs).contains("mcp_alias"));
}

#[test]
fn allowed_tool_ids_from_specs_collects_ids_aliases_and_deduplicates_collisions() {
    let specs = vec![
        crate::app::agent::tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        )
        .with_aliases(["read", "shared"]),
        crate::app::agent::tools::ToolSpec::new(
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
fn request_filter_uses_base_set_when_allowed_tools_is_not_a_non_empty_string_array() {
    let specs = vec![crate::app::agent::tools::ToolSpec::new(
        "file_read",
        "Read file",
        json!({"type": "object"}),
    )];
    let expected = HashSet::from(["file_read".to_string()]);

    for options in [
        json!({}),
        json!({"allowed_tools": null}),
        json!({"allowed_tools": "file_read"}),
        json!({"allowed_tools": []}),
        json!({"allowed_tools": [null, false, 42, "  "]}),
    ] {
        assert_eq!(allowed_tool_ids_for_request_from_specs(&specs, &options), expected);
    }
}

#[test]
fn request_filter_trims_names_accepts_aliases_and_discards_unknown_tools() {
    let specs = vec![
        crate::app::agent::tools::ToolSpec::new(
            "file_read",
            "Read file",
            json!({"type": "object"}),
        )
        .with_aliases(["read"]),
        crate::app::agent::tools::ToolSpec::new(
            "file_write",
            "Write file",
            json!({"type": "object"}),
        ),
    ];

    let allowed = allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"allowed_tools": [" read ", "file_write", "missing", "read"]}),
    );

    assert_eq!(allowed, HashSet::from(["read".to_string(), "file_write".to_string()]));
}

#[test]
fn response_preview_trims_scrubs_credentials_and_truncates() {
    let secret = "abcdefghijklmnopqrstuvwxyz";
    let preview = response_preview(&format!("  token={secret}  "));

    assert_eq!(preview, "token=abcd*[REDACTED]");
    assert!(!preview.contains(secret));

    let long_preview = response_preview(&"x".repeat(300));
    assert!(long_preview.len() < 300);
    assert!(long_preview.ends_with("..."));
}

#[test]
fn response_preview_handles_empty_input_and_exact_character_limit() {
    assert_eq!(response_preview(" \n\t "), "");

    let at_limit = "你".repeat(240);
    assert_eq!(response_preview(&at_limit), at_limit);

    let over_limit = "你".repeat(241);
    assert_eq!(response_preview(&over_limit), format!("{}...", "你".repeat(240)));
}

#[test]
fn tool_call_preview_sanitizes_large_or_sensitive_arguments() {
    let preview = tool_call_preview(
        "file_write",
        r#"{"path":"/tmp/example","content":"full file content should be omitted"}"#,
    );

    assert!(preview.starts_with("file_write("));
    assert!(preview.contains("<omitted"));
    assert!(!preview.contains("full file content should be omitted"));
}

#[test]
fn tool_call_preview_handles_empty_and_short_plain_arguments() {
    assert_eq!(tool_call_preview("file_read", "  "), "file_read()");
    assert_eq!(tool_call_preview("search", "  plain search query  "), "search(plain search query)");
}

#[test]
fn tool_call_preview_truncates_sanitized_arguments_at_character_boundary() {
    let preview = tool_call_preview("search", &"界".repeat(121));

    assert_eq!(preview, format!("search({}...)", "界".repeat(120)));
}

#[test]
fn acp_detection_accepts_test_flag_or_non_empty_agent() {
    assert!(is_acp_request(&json!({"acp_test": true})));
    assert!(is_acp_request(&json!({"acp_agent": "copilot"})));
    assert!(!is_acp_request(&json!({"acp_test": false, "acp_agent": "  "})));
    assert!(!is_acp_request(&json!({})));
}

#[test]
fn acp_detection_ignores_wrong_types_and_trims_agent_names() {
    assert!(!is_acp_request(&json!({"acp_test": "true"})));
    assert!(!is_acp_request(&json!({"acp_agent": 7})));
    assert!(is_acp_request(&json!({"acp_test": false, "acp_agent": "  copilot  "})));
    assert!(is_acp_request(&json!({"acp_test": true, "acp_agent": "  "})));
}

#[test]
fn structured_tool_calls_run_locally_only_outside_acp() {
    assert!(!should_execute_structured_tool_calls_locally(&json!({"acp_test": true})));
    assert!(!should_execute_structured_tool_calls_locally(&json!({"acp_agent": "copilot"})));
    assert!(should_execute_structured_tool_calls_locally(&json!({"acp_test": false})));
    assert!(should_execute_structured_tool_calls_locally(&json!({"acp_agent": "  "})));
}
