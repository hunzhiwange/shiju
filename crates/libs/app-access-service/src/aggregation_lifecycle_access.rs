use super::*;

/// Resolve the execution host for lifecycle work under the Gateway's tenant scope.
#[cfg(target_os = "macos")]
pub fn aggregation_lifecycle_host(
    path: &Path,
    tenant: &TenantScope,
    target: &str,
) -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, PathBuf), StatusCode> {
    let host =
        execution_context(Some(path), "gateway", tenant, target, None, Capability::Read)?.host;
    Ok((
        vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: host.id,
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        },
        host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
    ))
}

#[cfg(target_os = "macos")]
pub fn authorize_aggregation_lifecycle(
    path: &Path,
    project: &vw_api_transport::ProjectScope,
    host: &str,
    app: AppId,
    saved_principal: Option<&str>,
) -> Result<(PathBuf, PathBuf, String), StatusCode> {
    let authorization = execution_context(
        Some(path),
        "gateway",
        project.tenant_scope(),
        host,
        None,
        Capability::AggregationWrite(project.project_id(), app, true),
    )?;
    Ok((
        authorization.host.home,
        authorization.host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
        saved_principal.unwrap_or(&authorization.principal).to_owned(),
    ))
}
