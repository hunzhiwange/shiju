use super::McpCatalogStore;
use vw_api_mcp_management::{McpCatalogScope, McpConnectionCheckItem, McpFailureCode};
use vw_api_app_diagnostics::{DiagnosticJobSnapshot, DiagnosticProbeKind, DiagnosticResult};
use vw_api_transport::id::TaskId;
impl McpCatalogStore {
    /// Select results for the request currently shown by the caller, from an owner-authorized
    /// diagnostic snapshot. An edit invalidates old connection revisions, including completed jobs.
    pub fn current_connection_check_items(
        &self,
        scope: &McpCatalogScope,
        snapshot: &DiagnosticJobSnapshot,
        operation_id: &TaskId,
        generation: u64,
    ) -> Result<Vec<McpConnectionCheckItem>, McpFailureCode> {
        if snapshot.job.task_id != *operation_id || snapshot.job.kind != DiagnosticProbeKind::Mcp {
            return Ok(Vec::new());
        }
        let mut current = Vec::new();
        for result in snapshot.results.iter().flatten() {
            let DiagnosticResult::McpConnectionCheck {
                scope: result_scope,
                generation: result_generation,
                item,
            } = result
            else {
                continue;
            };
            if result_scope != scope || *result_generation != generation {
                continue;
            }
            match self.current_connection_revision(scope, &item.id) {
                Ok(revision) if revision == item.connection_revision => {
                    current.push(item.clone());
                }
                Ok(_) | Err(McpFailureCode::NotFound) => {}
                Err(code) => return Err(code),
            }
        }
        Ok(current)
    }
}
