use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Frame {
    #[serde(rename = "type")]
    pub kind: String,
    pub msg_id: Option<String>,
    pub call_id: Option<String>,
    pub text: Option<String>,
    pub output: Option<String>,
    pub name: Option<String>,
    pub tools: Option<Vec<String>>,
    pub status: Option<String>,
    pub tool: Option<Tool>,
    pub version: Option<String>,
    pub capabilities: Option<Capabilities>,
}
#[derive(Debug, Deserialize)]
pub struct Tool {
    pub name: String,
    pub args: serde_json::Value,
    pub category: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Capabilities {
    pub tool_approval: bool,
    pub current_mode: String,
}
