//! Backup inventory is scoped to the authenticated gateway principal.
use super::{GatewayClient, agent_apps_read_api::AgentAppsReadError};
use vw_api_types::{ProjectScope, agent_apps::data_management::DataBackupSummary};
impl GatewayClient {
    pub async fn data_backup_inventory(&self, project: &ProjectScope, host: &str)
        -> Result<Vec<DataBackupSummary>, AgentAppsReadError> {
        if project.tenant_scope() != self.endpoint().tenant_scope() || host.trim().is_empty() {
            return Err(AgentAppsReadError::Transport("invalid backup inventory scope".into()));
        }
        let request = self.native_transport().map_err(AgentAppsReadError::Transport)?
            .get(format!("{}/v1/agent-apps/data-backup-inventory", self.endpoint().base_url()))
            .query(&[("projectId", project.project_id().as_ref()), ("targetHostId", host)]);
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await
            .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
        if !status.is_success() {
            return Err(AgentAppsReadError::Http { status: status.as_u16(),
                body: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null) });
        }
        let summaries: Vec<DataBackupSummary> = serde_json::from_slice(&bytes)
            .map_err(|e| AgentAppsReadError::Decode(e.to_string()))?;
        if summaries.iter().any(|entry| &entry.target.scope != project || entry.target.target_host_id != host) {
            return Err(AgentAppsReadError::Decode("backup inventory scope mismatch".into()));
        }
        Ok(summaries)
    }
}
