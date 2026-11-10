//! Built-in MCP presets and their disabled catalog drafts.

use super::validation::{ValidationError, validate_connection, validate_new_name};
use vw_api_types::mcp_management::{McpConnectionDefinition, McpRecordId};

pub struct McpPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub tags: &'static [&'static str],
    pub homepage: &'static str,
    pub documentation: &'static str,
    command: &'static str,
    args: &'static [&'static str],
}

pub struct McpPresetDraft {
    pub preset: &'static McpPreset,
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

const MCP_SERVERS_HOMEPAGE: &str = "https://github.com/modelcontextprotocol/servers";

const PRESETS: &[McpPreset] = &[
    McpPreset {
        id: "fetch",
        name: "mcp-server-fetch",
        tags: &["stdio", "http", "web"],
        homepage: MCP_SERVERS_HOMEPAGE,
        documentation: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
        command: "uvx",
        args: &["mcp-server-fetch"],
    },
    McpPreset {
        id: "time",
        name: "mcp-server-time",
        tags: &["stdio", "time", "utility"],
        homepage: MCP_SERVERS_HOMEPAGE,
        documentation: "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
        command: "uvx",
        args: &["mcp-server-time"],
    },
    McpPreset {
        id: "memory",
        name: "@modelcontextprotocol/server-memory",
        tags: &["stdio", "memory", "graph"],
        homepage: MCP_SERVERS_HOMEPAGE,
        documentation: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
        command: "npx",
        args: &["-y", "@modelcontextprotocol/server-memory"],
    },
    McpPreset {
        id: "sequential-thinking",
        name: "@modelcontextprotocol/server-sequential-thinking",
        tags: &["stdio", "thinking", "reasoning"],
        homepage: MCP_SERVERS_HOMEPAGE,
        documentation: "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
        command: "npx",
        args: &["-y", "@modelcontextprotocol/server-sequential-thinking"],
    },
    McpPreset {
        id: "context7",
        name: "@upstash/context7-mcp",
        tags: &["stdio", "docs", "search"],
        homepage: "https://context7.com",
        documentation: "https://github.com/upstash/context7/blob/master/README.md",
        command: "npx",
        args: &["-y", "@upstash/context7-mcp"],
    },
];

pub fn list_presets() -> &'static [McpPreset] {
    PRESETS
}

/// Build a disabled draft using record IDs from the caller's authorized catalog scope.
/// An unknown preset yields `None`. Saving still requires an atomic uniqueness check.
pub fn create_preset_draft(
    id: &str,
    existing: &[McpRecordId],
) -> Result<Option<McpPresetDraft>, ValidationError> {
    let Some(preset) = list_presets().iter().find(|preset| preset.id == id) else {
        return Ok(None);
    };
    validate_new_name(preset.id, existing)?;
    let connection = McpConnectionDefinition::Stdio {
        command: preset.command.to_owned(),
        args: preset.args.iter().map(|arg| (*arg).to_owned()).collect(),
        env: Default::default(),
        cwd: None,
    };
    validate_connection(&connection)?;
    Ok(Some(McpPresetDraft { preset, connection, enabled: false }))
}
