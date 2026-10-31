//! Ephemeral MCP host form data; never persisted or exposed as model tools.

/// An ephemeral, source-bound MCP form. It is never a model tool or persisted message.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpFormRequest {
    pub id: String,
    pub session_id: String,
    pub server_name: String,
    pub connection_id: String,
    pub server_request_id: serde_json::Value,
    pub message: String,
    pub requested_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpFormAction {
    Accept,
    Decline,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFormResponse {
    pub action: McpFormAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFormHostOpen {
    pub session_id: String,
    pub workspace: Option<String>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFormHostRef {
    pub host_id: String,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct McpFormHostOpened {
    pub host_id: String,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpFormHostReply {
    pub host_id: String,
    pub request: McpFormRequest,
    pub response: McpFormResponse,
}
impl std::fmt::Debug for McpFormHostReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpFormHostReply")
            .field("action", &self.response.action)
            .finish_non_exhaustive()
    }
}
