use super::{
    presets::{create_preset_draft, list_presets},
    validation::{ValidationError, validate_connection},
};
use vw_api_types::mcp_management::{McpConnectionDefinition, McpRecordId};

#[test]
fn cc_mcp_t05_catalog_lists_five_snapshot_presets() {
    let presets = list_presets();
    assert_eq!(
        presets.iter().map(|preset| preset.id).collect::<Vec<_>>(),
        ["fetch", "time", "memory", "sequential-thinking", "context7"]
    );
    let expected = [
        (
            "mcp-server-fetch",
            ["stdio", "http", "web"],
            "https://github.com/modelcontextprotocol/servers",
            "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
        ),
        (
            "mcp-server-time",
            ["stdio", "time", "utility"],
            "https://github.com/modelcontextprotocol/servers",
            "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
        ),
        (
            "@modelcontextprotocol/server-memory",
            ["stdio", "memory", "graph"],
            "https://github.com/modelcontextprotocol/servers",
            "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
        ),
        (
            "@modelcontextprotocol/server-sequential-thinking",
            ["stdio", "thinking", "reasoning"],
            "https://github.com/modelcontextprotocol/servers",
            "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
        ),
        (
            "@upstash/context7-mcp",
            ["stdio", "docs", "search"],
            "https://context7.com",
            "https://github.com/upstash/context7/blob/master/README.md",
        ),
    ];
    for (preset, (name, tags, homepage, documentation)) in presets.iter().zip(expected) {
        assert_eq!(preset.name, name, "{}", preset.id);
        assert_eq!(preset.tags, tags, "{}", preset.id);
        assert_eq!(preset.homepage, homepage, "{}", preset.id);
        assert_eq!(preset.documentation, documentation, "{}", preset.id);
    }
}

#[test]
fn cc_mcp_t05_presets_create_disabled_valid_stdio_drafts() {
    let expected: [(&str, &str, &[&str]); 5] = [
        ("fetch", "uvx", &["mcp-server-fetch"]),
        ("time", "uvx", &["mcp-server-time"]),
        ("memory", "npx", &["-y", "@modelcontextprotocol/server-memory"]),
        ("sequential-thinking", "npx", &["-y", "@modelcontextprotocol/server-sequential-thinking"]),
        ("context7", "npx", &["-y", "@upstash/context7-mcp"]),
    ];
    let existing = [McpRecordId("unrelated".into())];
    for (id, expected_command, expected_args) in expected {
        let draft = create_preset_draft(id, &existing)
            .unwrap()
            .unwrap_or_else(|| panic!("missing preset: {id}"));
        assert_eq!(draft.preset.id, id);
        assert!(!draft.enabled, "{id}");
        assert_eq!(validate_connection(&draft.connection), Ok(()));
        let McpConnectionDefinition::Stdio { command, args, env, cwd } = draft.connection else {
            panic!("{id} should use stdio");
        };
        assert_eq!(command, expected_command, "{id}");
        assert_eq!(args, expected_args, "{id}");
        assert!(env.is_empty(), "{id}");
        assert!(cwd.is_none(), "{id}");
    }
}

#[test]
fn cc_mcp_t05_existing_record_id_rejects_preset_draft() {
    let existing = [McpRecordId("fetch".into())];
    assert!(matches!(create_preset_draft("fetch", &existing), Err(ValidationError::NameConflict)));
}

#[test]
fn cc_mcp_t05_unknown_id_has_no_draft() {
    assert!(create_preset_draft("missing", &[]).unwrap().is_none());
}
