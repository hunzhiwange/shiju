use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub struct McpNewServerState {
    pub name_input: String,
    pub transport: McpNewServerTransport,
    pub command_input: String,
    pub args_input: String,
    pub url_input: String,
    pub timeout_input: String,
    pub enabled: bool,
    pub required: bool,
}

impl Default for McpNewServerState {
    fn default() -> Self {
        Self {
            name_input: String::new(),
            transport: McpNewServerTransport::Stdio,
            command_input: String::new(),
            args_input: String::new(),
            url_input: String::new(),
            timeout_input: "30".to_string(),
            enabled: true,
            required: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpNewServerTransport {
    Stdio,
    Http,
    Sse,
}

impl McpNewServerTransport {
    pub const ALL: [Self; 3] = [Self::Stdio, Self::Http, Self::Sse];

    pub fn label(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
            Self::Sse => "sse",
        }
    }
}

impl std::fmt::Display for McpNewServerTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub active_tab: McpSettingsTab,
    pub editor: text_editor::Content,
    pub runtime_overrides: Vec<serde_json::Value>,
    pub new_server_expanded: bool,
    pub editing_server_index: Option<usize>,
    pub expanded_previews: std::collections::BTreeSet<String>,
    pub new_server: McpNewServerState,
    pub loading: bool,
    pub inspect_loading: bool,
    pub saving: bool,
    pub server_count: usize,
    pub inspect: Option<vw_api_mcp_inspect::McpInspectResponse>,
    pub inspect_error: Option<String>,
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
    pub status_message: Option<String>,
    /// 待删除确认的 MCP 服务器索引。
    pub delete_pending: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSettingsTab {
    Servers,
    Runtime,
    Json,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: McpSettingsTab::Servers,
            editor: text_editor::Content::with_text("[]"),
            runtime_overrides: Vec::new(),
            new_server_expanded: false,
            editing_server_index: None,
            expanded_previews: std::collections::BTreeSet::new(),
            new_server: McpNewServerState::default(),
            loading: false,
            inspect_loading: false,
            saving: false,
            server_count: 0,
            inspect: None,
            inspect_error: None,
            save_error: None,
            save_error_translation: None,
            status_message: None,
            delete_pending: None,
        }
    }
}
