//! # Dify DSL 模型
//!
//! 该模块定义用于反序列化 Dify DSL 的中间结构体，供加载流程做结构化解析。

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct DifyWorkflowFile {
    pub(super) app: Option<DifyApp>,
    pub(super) workflow: Option<DifyWorkflowShell>,
    pub(super) graph: Option<DifyGraph>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DifyApp {
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) icon: Option<String>,
    pub(super) icon_background: Option<String>,
    pub(super) mode: Option<String>,
    pub(super) use_icon_as_answer_icon: Option<bool>,
    pub(super) max_active_requests: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DifyWorkflowShell {
    pub(super) graph: DifyGraph,
    #[serde(default)]
    pub(super) environment_variables: Vec<DifyEnvironmentVariable>,
    #[serde(default)]
    pub(super) conversation_variables: Vec<DifyConversationVariable>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DifyGraph {
    #[serde(default)]
    pub(super) nodes: Vec<DifyNode>,
    #[serde(default)]
    pub(super) edges: Vec<DifyEdge>,
    pub(super) viewport: Option<DifyViewport>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub(super) struct DifyPoint {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DifyNode {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) renderer_type: Option<String>,
    pub(super) data: DifyNodeData,
    pub(super) position: Option<DifyPoint>,
    pub(super) position_absolute: Option<DifyPoint>,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
    pub(super) parent_id: Option<String>,
    pub(super) selected: Option<bool>,
    pub(super) source_position: Option<String>,
    pub(super) target_position: Option<String>,
    pub(super) z_index: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct DifyNodeData {
    #[serde(default)]
    pub(super) title: String,
    #[serde(default)]
    pub(super) desc: String,
    #[serde(rename = "type", default)]
    pub(super) block_type: String,
    #[serde(default)]
    pub(super) error_strategy: String,
    #[serde(default)]
    pub(super) cases: Vec<DifyCase>,
    #[serde(default)]
    pub(super) classes: Vec<DifyCase>,
    pub(super) selected: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct DifyCase {
    pub(super) case_id: Option<String>,
    pub(super) id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DifyEdge {
    pub(super) id: Option<String>,
    pub(super) source: String,
    pub(super) target: String,
    pub(super) source_handle: Option<String>,
    pub(super) target_handle: Option<String>,
    pub(super) selected: Option<bool>,
    pub(super) data: Option<DifyEdgeData>,
    pub(super) z_index: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DifyEdgeData {
    #[serde(default)]
    pub(super) source_type: String,
    #[serde(default)]
    pub(super) target_type: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(super) struct DifyEnvironmentVariable {
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) name: String,
    #[serde(default)]
    pub(super) value_type: String,
    #[serde(default)]
    pub(super) value: Value,
    #[serde(default)]
    pub(super) description: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(super) struct DifyConversationVariable {
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) name: String,
    #[serde(default)]
    pub(super) value_type: String,
    #[serde(default)]
    pub(super) value: Value,
    #[serde(default)]
    pub(super) description: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct DifyViewport {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) zoom: f32,
}
