use serde::{Deserialize, Serialize};

/// Requested intent is retained when a published member disappears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexSettingsValues {
    pub subagent_model_id: Option<String>,
    pub subagent_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub auto_review_model_id: Option<String>,
    pub classic_tools: bool,
}
impl Default for CodexSettingsValues {
    fn default() -> Self {
        Self {
            subagent_model_id: None,
            subagent_effort: None,
            auto_review_model_id: None,
            classic_tools: true,
        }
    }
}
