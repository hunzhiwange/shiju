//! Pure MCP specification mapping; no application host, I/O or environment state.

use super::{mcp_tool_id, mcp_tools_to_specs};
use rmcp::model::{JsonObject, Tool as McpToolDefinition, ToolAnnotations};
use serde_json::json;

fn schema(value: serde_json::Value) -> JsonObject {
    match value {
        serde_json::Value::Object(object) => object,
        _ => panic!("test schema must be a JSON object"),
    }
}

fn tool(name: &str, description: &str, input_schema: serde_json::Value) -> McpToolDefinition {
    McpToolDefinition::new(name.to_string(), description.to_string(), schema(input_schema))
}


#[test]
fn mcp_tool_spec_ids_are_stable_and_prefixed() {
    let specs = mcp_tools_to_specs(
        "GitHub Server",
        None,
        &[tool(
            "Search Issues",
            "Search repository issues.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
        )],
        &[],
    )
    .expect("tool spec mapping should succeed");

    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "mcp__github_server__search_issues_7c7d4f69a793c4d1");
    assert_eq!(specs[0].name, specs[0].id);
    assert_eq!(specs[0].display_name, "GitHub Server: Search Issues");
    assert_eq!(mcp_tool_id("GitHub Server", "Search Issues").unwrap(), specs[0].id);
}

#[test]
fn mcp_tool_spec_maps_schema_and_annotations() {
    let input_schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "path": { "type": "string" }
        },
        "required": ["path"]
    });
    let annotations = ToolAnnotations::new().read_only(true).destructive(false).idempotent(true);
    let spec = mcp_tools_to_specs(
        "docs",
        Some("Use for public documentation lookups only.\nBearer abc.def must stay private."),
        &[tool("fetch_page", "Fetch a documentation page.", input_schema.clone())
            .annotate(annotations)],
        &[],
    )
    .expect("tool spec mapping should succeed")
    .pop()
    .expect("spec should exist");

    assert_eq!(spec.input_schema, input_schema);
    assert_eq!(spec.parameters, spec.input_schema);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert!(spec.description.contains("Fetch a documentation page."));
    assert!(
        spec.description
            .contains("Server instructions: Use for public documentation lookups only.")
    );
    assert!(!spec.description.contains("Bearer abc.def"));
}

#[test]
fn mcp_tool_spec_uses_fallback_description_and_truncates_long_descriptions() {
    let fallback = mcp_tools_to_specs(
        "docs",
        None,
        &[tool("blank", " \n\t ", json!({ "type": "object" }))],
        &[],
    )
    .expect("blank description should map")
    .pop()
    .expect("fallback spec");
    assert_eq!(fallback.description, "Remote MCP tool.");

    let long_description = "a".repeat(1_400);
    let long_instructions = "Use this server for documentation lookups. ".repeat(20);
    let spec = mcp_tools_to_specs(
        "docs",
        Some(&long_instructions),
        &[tool("long", &long_description, json!({ "type": "object" }))],
        &[],
    )
    .expect("long description should map")
    .pop()
    .expect("long spec");

    assert_eq!(spec.description.chars().count(), 1_200);
    assert!(spec.description.ends_with("..."));
}

#[test]
fn mcp_tool_spec_rejects_colliding_ids() {
    let error = mcp_tools_to_specs(
        "docs",
        None,
        &[
            tool("fetch_page", "Fetch a documentation page.", json!({ "type": "object" })),
            tool("fetch_page", "Fetch a page by alias.", json!({ "type": "object" })),
        ],
        &["mcp__other__tool"],
    )
    .expect_err("normalized tool id collision should be rejected");

    assert!(error.to_string().contains("duplicate MCP tool id 'mcp__docs__fetch_page'"));

    let builtin_error = mcp_tools_to_specs(
        "docs",
        None,
        &[tool("fetch", "Fetch a page.", json!({ "type": "object" }))],
        &["mcp__docs__fetch"],
    )
    .expect_err("colliding reserved ids should be rejected");

    assert!(
        builtin_error
            .to_string()
            .contains("MCP tool id 'mcp__docs__fetch' collides with an existing tool")
    );
}

#[test]
fn mcp_tool_id_rejects_segments_without_ascii_alphanumeric_characters() {
    let server_error = mcp_tool_id("---", "fetch").expect_err("blank server id segment");
    assert!(server_error.to_string().contains("invalid MCP server name '---'"));

    let tool_error = mcp_tool_id("docs", "你好").expect_err("blank tool id segment");
    assert!(tool_error.to_string().contains("invalid MCP tool name '你好'"));
}

#[test]
fn codex_plugin_compat_t19_calls_exact_namespaced_tool() {
    let a = mcp_tool_id("package-a", "Read.File").unwrap();
    let b = mcp_tool_id("package-a", "read_file").unwrap();
    assert_ne!(a, b, "different raw names must remain independently callable");
}
#[test]
fn codex_plugin_compat_t19_rejects_invalid_input() {
    let id = mcp_tool_id("server", &"x".repeat(200)).unwrap();
    assert!(id.len() <= 64, "model callable identifier must fit the name limit");
}
#[test]
fn codex_plugin_compat_t19_preserves_isolation() {
    assert_ne!(
        mcp_tool_id("a-b", "tool").unwrap(),
        mcp_tool_id("a_b", "tool").unwrap(),
        "distinct source namespaces must not collapse"
    );
}
