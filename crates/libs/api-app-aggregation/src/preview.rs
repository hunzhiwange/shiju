//! Aggregation and native directory resources scoped to a project.
use vw_api_transport::id::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationQuery {
    #[serde(default = "vw_api_transport::local_host_id")]
    pub target_host_id: String,
    #[serde(default = "vw_api_transport::local_project_id")]
    pub project_id: ProjectId,
}
