use super::*;
use crate::*;
use agent_client_protocol::{McpServerHttp, McpServerSse, McpServerStdio};
use std::path::PathBuf;

#[test]
fn server_helpers_preserve_transport_details() {
    let http = McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"));
    assert_eq!(server_name(&http), "gateway");
    assert_eq!(
        server_summary(&http),
        json!({
            "name": "gateway",
            "transport": "http",
            "url": "https://example.com/mcp"
        })
    );

    let sse = McpServer::Sse(McpServerSse::new("events", "https://example.com/sse"));
    assert_eq!(server_name(&sse), "events");
    assert_eq!(
        server_summary(&sse),
        json!({
            "name": "events",
            "transport": "sse",
            "url": "https://example.com/sse"
        })
    );

    let stdio = McpServer::Stdio(
        McpServerStdio::new("local", PathBuf::from("npx")).args(vec!["mcp".into()]),
    );
    assert_eq!(server_name(&stdio), "local");
    assert_eq!(
        server_summary(&stdio),
        json!({
            "name": "local",
            "transport": "stdio",
            "command": "npx",
            "args": ["mcp"]
        })
    );
}

#[test]
fn mcp_common_server_lookup_rejects_unknown_names() {
    let servers = vec![
        McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp")),
        McpServer::Sse(McpServerSse::new("events", "https://example.com/sse")),
    ];

    let found = find_server_by_name(&servers, "events").expect("known server should be found");
    assert_eq!(server_name(found), "events");

    let error = find_server_by_name(&servers, "missing")
        .expect_err("unknown server name should be rejected");
    assert_eq!(error.to_string(), "unknown MCP server 'missing'");
}

#[test]
fn mcp_common_server_lookup_trims_names_and_rejects_blank_input() {
    let servers = vec![McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"))];

    let found = find_server_by_name(&servers, "  gateway  ")
        .expect("surrounding whitespace should be ignored");
    assert_eq!(server_name(found), "gateway");

    let error =
        find_server_by_name(&servers, " \t ").expect_err("a blank server name should be rejected");
    assert_eq!(error.to_string(), "MCP server name must not be empty");
}

#[test]
fn mcp_tool_policy_honors_allow_lists_and_deny_precedence() {
    let mut management = acp_config::McpServerManagement::default_for_server("gateway");
    assert!(mcp_tool_allowed(&management, "echo", "gateway__echo"));

    management.allowed_tools = vec!["echo".to_string(), "gateway__sum".to_string()];
    assert!(mcp_tool_allowed(&management, "echo", "gateway__echo"));
    assert!(mcp_tool_allowed(&management, "sum", "gateway__sum"));
    assert!(!mcp_tool_allowed(&management, "hidden", "gateway__hidden"));

    management.denied_tools = vec!["echo".to_string(), "gateway__sum".to_string()];
    assert!(!mcp_tool_allowed(&management, "echo", "gateway__echo"));
    assert!(!mcp_tool_allowed(&management, "sum", "gateway__sum"));
}

#[test]
fn mcp_auth_method_names_are_sorted_without_exposing_values() {
    let auth = HashMap::from([
        ("oauth".to_string(), "opaque-oauth-value".to_string()),
        ("api_key".to_string(), "opaque-api-value".to_string()),
    ]);

    assert_eq!(auth_method_names(&auth), vec!["api_key", "oauth"]);
    assert!(auth_method_names(&HashMap::new()).is_empty());
}

#[test]
fn mcp_server_transport_reports_each_supported_transport() {
    let http = McpServer::Http(McpServerHttp::new("http", "https://example.com/mcp"));
    let sse = McpServer::Sse(McpServerSse::new("sse", "https://example.com/sse"));
    let stdio = McpServer::Stdio(McpServerStdio::new("stdio", "mcp-command"));

    assert_eq!(server_transport(&http), "http");
    assert_eq!(server_transport(&sse), "sse");
    assert_eq!(server_transport(&stdio), "stdio");
}

#[test]
fn mcp_resource_contents_json_preserves_text_blob_and_empty_inputs() {
    let contents = vec![
        ResourceContents::TextResourceContents {
            uri: "file:///notes.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: "hello".to_string(),
            meta: None,
        },
        ResourceContents::BlobResourceContents {
            uri: "file:///image.bin".to_string(),
            mime_type: None,
            blob: "AAEC".to_string(),
            meta: None,
        },
    ];

    assert_eq!(
        resource_contents_json(&contents),
        json!([
            {
                "uri": "file:///notes.txt",
                "mime_type": "text/plain",
                "text": "hello",
                "kind": "text"
            },
            {
                "uri": "file:///image.bin",
                "mime_type": null,
                "blob": "AAEC",
                "kind": "blob"
            }
        ])
    );
    assert_eq!(resource_contents_json(&[]), json!([]));
}

#[test]
fn mcp_server_override_parser_trims_values_and_rounds_fractional_timeout() {
    let parsed = parse_mcp_server_management_override(
        &json!({
            "name": "  plugin-mcp  ",
            "enabled": true,
            "required": false,
            "timeout": 1.2345,
            "allowedTools": ["  demo_echo  "],
            "deniedTools": []
        }),
        "vibewindow.json",
        0,
    )
    .expect("valid override should parse");

    assert_eq!(parsed.name, "plugin-mcp");
    assert_eq!(parsed.enabled, Some(true));
    assert_eq!(parsed.required, Some(false));
    assert_eq!(parsed.timeout_ms, Some(Some(1_235)));
    assert_eq!(parsed.allowed_tools, Some(vec!["demo_echo".to_string()]));
    assert_eq!(parsed.denied_tools, Some(Vec::new()));
}

#[test]
fn mcp_server_override_parser_rejects_invalid_fields_with_paths() {
    let cases = [
        (json!([]), "expected object"),
        (json!({ "name": "  " }), ".name: expected non-empty string"),
        (json!({ "name": "plugin-mcp", "enabled": "yes" }), ".enabled: expected boolean"),
        (
            json!({ "name": "plugin-mcp", "timeout": 0 }),
            ".timeout: expected positive seconds or null",
        ),
        (
            json!({ "name": "plugin-mcp", "allowedTools": "demo_echo" }),
            ".allowedTools: expected array",
        ),
        (
            json!({ "name": "plugin-mcp", "deniedTools": ["  "] }),
            ".deniedTools[0]: expected non-empty string",
        ),
    ];

    for (index, (value, expected)) in cases.into_iter().enumerate() {
        let error = parse_mcp_server_management_override(&value, "vibewindow.json", index)
            .expect_err("invalid override should be rejected");
        let message = error.to_string();
        assert!(message.contains(&format!("mcpServerOverrides[{index}] in vibewindow.json")));
        assert!(message.contains(expected), "unexpected error: {message}");
    }
}

#[test]
fn mcp_server_override_parser_rejects_wrong_timeout_and_tool_element_types() {
    let cases = [
        (
            json!({ "name": "plugin-mcp", "timeout": "fast" }),
            ".timeout: expected positive seconds or null",
        ),
        (
            json!({ "name": "plugin-mcp", "allowedTools": [7] }),
            ".allowedTools[0]: expected non-empty string",
        ),
    ];

    for (index, (value, expected)) in cases.into_iter().enumerate() {
        let error = parse_mcp_server_management_override(&value, "vibewindow.json", index)
            .expect_err("wrongly typed override field should be rejected");
        assert!(error.to_string().contains(expected));
    }
}
