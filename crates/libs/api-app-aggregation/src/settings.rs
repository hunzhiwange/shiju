use vw_api_transport::id::ProjectId;
use serde::{Deserialize, Serialize};
use crate::*;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSettingsChoice {
    pub model_id: String,
    pub label: String,
    pub efforts: Vec<vw_api_chat_identity::ReasoningEffortDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAggregationSettings {
    pub revision: String,
    pub requested: CodexSettingsValues,
    pub effective: CodexSettingsValues,
    pub choices: Vec<CodexSettingsChoice>,
    pub notices: Vec<AggregationNotice>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexSettingsRequest {
    #[serde(default = "vw_api_transport::local_host_id")]
    pub target_host_id: String,
    #[serde(default = "vw_api_transport::local_project_id")]
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub requested: CodexSettingsValues,
}
