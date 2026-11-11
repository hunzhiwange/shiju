use super::*;

/// Executor-only lifecycle resolution from the administrator's policy. No HTTP
/// route accepts a principal ID in place of a credential. Saved intent selects
/// the project; every resume/detach rechecks policy grants, retaining the saved
/// actor when the publication carries one.
#[cfg(target_os = "macos")]
pub(crate) fn aggregation_lifecycle_host(
    path: &Path,
    tenant: &TenantScope,
    target: &str,
) -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, PathBuf), StatusCode> {
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !policy.principals.iter().flat_map(|p| &p.grants).any(|g| {
        g.platform_id == tenant.platform_id().as_ref()
            && g.company_id == tenant.company_id().as_ref()
            && g.host_id == target
            && (g.routing_read || g.routing_manage)
    }) {
        return Err(StatusCode::FORBIDDEN);
    }
    let host = policy.hosts.into_iter().find(|h| h.id == target).ok_or(StatusCode::NOT_FOUND)?;
    if !host.online || !matches!(host.binding, Binding::Local) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
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
pub(crate) fn authorize_aggregation_lifecycle(
    path: &Path,
    project: &vw_api_types::id::ProjectScope,
    host: &str,
    app: AppId,
    saved_principal: Option<&str>,
) -> Result<(PathBuf, PathBuf, String), StatusCode> {
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    for principal in &policy.principals {
        if saved_principal.is_some_and(|id| id != principal.id) {
            continue;
        }
        // Reuse the complete original capability predicate and host checks.
        if let Ok(authorization) = authorize_digest(
            Some(path),
            &principal.token_sha256,
            project.tenant_scope(),
            host,
            None,
            Capability::AggregationWrite(project.project_id(), app, true),
        ) {
            if authorization.principal != principal.id {
                continue;
            }
            return Ok((
                authorization.host.home,
                authorization.host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
                authorization.principal,
            ));
        }
    }
    Err(StatusCode::FORBIDDEN)
}
