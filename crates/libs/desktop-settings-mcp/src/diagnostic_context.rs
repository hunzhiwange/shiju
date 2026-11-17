pub use desktop_apps_ui::agent_apps_diagnostics::mcp_context::*;

/// Project the selected authorized catalog into a revision-bound diagnostic scope.
pub fn current(
    scope: Option<crate::management_context::Context>,
    project: Option<&str>,
    selected: Option<&desktop_apps_ui::agent_apps_diagnostics::mcp_state::AppChoice>,
    token: &str,
    endpoint: Option<&desktop_provider_apps::AgentAppsContext>,
    applications: impl IntoIterator<Item = vw_api_app_identity::AppId>,
    catalog: Option<&Result<vw_api_mcp_management::McpCatalogResponse, String>>,
) -> Option<Context> {
    let scope = scope?;
    let project = project.filter(|p| !p.is_empty())?;
    let choice = selected?;
    if scope.host.is_empty()

        || endpoint != Some(&scope.endpoint)
        || !applications.into_iter().any(|id| id == choice.0)
    {
        return None;
    }
    let connections = catalog?.as_ref().ok()?.records.clone();
    Some(Context {
        records: connections.iter().map(|r| r.record.clone()).collect(),
        connections,
        query: vw_api_app_diagnostics::DiagnosticScopeQuery {
            project_id: project.into(),
            target_host_id: scope.host,
            app_id: choice.0,
        },
        gateway: scope.endpoint.gateway,
        tenant: scope.endpoint.tenant,
        project_path: scope.endpoint.project,
        credentials_generation: scope.credentials_generation,
    })
}
