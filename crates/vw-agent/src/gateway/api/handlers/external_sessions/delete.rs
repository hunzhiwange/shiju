//! Authorized destructive operations share native dispatch and host lifecycle locks.
use super::catalog::{Policy, Snapshots};
use axum::{
    Extension, Json,
    http::{HeaderMap, StatusCode},
};
use std::{collections::HashMap, io};
use vw_agent_apps::{
    apps::lifecycle::coordinator::ToolLifecycleCoordinator,
    sessions::{cache, source::SourceRegistry},
};
use vw_api_types::{TenantScope, agent_apps::AppId, external_session::*};

pub(super) async fn single(
    Extension(policy): Extension<Policy>,
    Extension(snapshots): Extension<Snapshots>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<DeleteSessionRequest>,
) -> Result<Json<DeleteSessionOutcome>, StatusCode> {
    let mut results = execute(policy, snapshots, tenant, headers, vec![request]).await?;
    Ok(Json(results.remove(0)))
}
pub(super) async fn batch(
    Extension(policy): Extension<Policy>,
    Extension(snapshots): Extension<Snapshots>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(requests): Json<Vec<DeleteSessionRequest>>,
) -> Result<Json<Vec<DeleteSessionOutcome>>, StatusCode> {
    execute(policy, snapshots, tenant, headers, requests).await.map(Json)
}
async fn execute(
    policy: Policy,
    snapshots: Snapshots,
    tenant: TenantScope,
    headers: HeaderMap,
    requests: Vec<DeleteSessionRequest>,
) -> Result<Vec<DeleteSessionOutcome>, StatusCode> {
    if requests.is_empty() || requests.len() > 200 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut tools = Vec::new();
    for request in &requests {
        if request.identity.scope.tenant_scope() != &tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        vw_agent_apps::sessions::identity::validate_identity(&request.identity)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let app = app(request.identity.provider);
        if !tools.contains(&app) {
            tools.push(app);
        }
    }
    ToolLifecycleCoordinator
        .run(tools, move || {
            // Authorization, source discovery and activity checks all run while the
            // real host write guard is retained, even if the HTTP waiter disconnects.
            let mut registries = HashMap::new();
            for request in &requests {
                registries
                    .entry(request.identity.clone())
                    .or_insert_with(|| registry(&policy, &headers, &request.identity));
            }
            // Invalidate before dispatch: a native adapter can fail after partial cleanup.
            for request in &requests {
                if registries.get(&request.identity).is_some_and(|r| r.is_ok()) {
                    snapshots
                        .deletion_attempted(&request.identity)
                        .map_err(|_| "catalog unavailable".to_owned())?;
                }
            }
            let outcomes = vw_agent_apps::sessions::delete_batch::delete_sessions(
                &requests,
                |id| match registries.get(id) {
                    Some(Ok(registry)) => Ok(registry),
                    Some(Err(kind)) => Err(io::Error::new(*kind, "session deletion refused")),
                    None => Err(io::Error::new(io::ErrorKind::PermissionDenied, "unbound session")),
                },
                cache::global(),
                active,
                // This synchronous endpoint has no cancellation operation ID;
                // disconnects must not interrupt an in-flight native deletion.
                || false,
                &uuid::Uuid::new_v4().to_string(),
            );
            for request in &requests {
                if registries.get(&request.identity).is_some_and(|r| r.is_ok()) {
                    snapshots
                        .deletion_attempted(&request.identity)
                        .map_err(|_| "catalog unavailable".to_owned())?;
                }
            }
            Ok(outcomes)
        })
        .await
        .map_err(|_| StatusCode::CONFLICT)
}
fn active(id: &ExternalSessionIdentity) -> io::Result<bool> {
    #[cfg(target_os = "macos")]
    {
        vw_agent_apps::sessions::activity::is_active(id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = id;
        Err(io::Error::new(io::ErrorKind::Unsupported, "activity probe unsupported"))
    }
}
fn registry(
    policy: &Policy,
    headers: &HeaderMap,
    id: &ExternalSessionIdentity,
) -> Result<SourceRegistry, io::ErrorKind> {
    let (_, home, instances) = super::super::agent_apps::authorize_external_history_delete(
        policy.0.as_deref(),
        headers,
        &id.scope,
        &id.target_host,
    )
    .map_err(|status| {
        if status == StatusCode::FORBIDDEN || status == StatusCode::UNAUTHORIZED {
            io::ErrorKind::PermissionDenied
        } else {
            io::ErrorKind::Other
        }
    })?;
    let instance = instances
        .into_iter()
        .find(|i| i.provider == id.provider && i.instance_id == id.instance_id)
        .ok_or(io::ErrorKind::PermissionDenied)?;
    let (sessions, _) =
        super::scan::scan(&instance.root, &home, id).map_err(|_| io::ErrorKind::Other)?;
    let session = sessions
        .into_iter()
        .find(|session| session.identity == *id)
        .ok_or(io::ErrorKind::NotFound)?;
    use vw_agent_apps::sessions::providers::{omp, pi};
    let root = match id.provider {
        SessionProvider::Pi => match pi::session_roots(&pi::PiSessionContext {
            home: &home,
            configured_agent_dir: Some(&instance.root),
            env_agent_dir: None,
            env_session_dir: None,
            project_dir: None,
        }) {
            pi::SessionRootResolution::Available { root, .. } => root,
            _ => return Err(io::ErrorKind::Unsupported),
        },
        SessionProvider::Omp => match omp::session_roots(&omp::OmpSessionContext {
            agent_dir: &instance.root,
            native_agent_dir: &instance.root,
            profile: None,
            xdg_data_home: None,
            env_session_dir: None,
        }) {
            pi::SessionRootResolution::Available { root, .. } => root,
            _ => return Err(io::ErrorKind::Unsupported),
        },
        _ => instance.root,
    };
    let relative = match &session.identity.source {
        SessionSource::File { relative_path } => relative_path,
        SessionSource::Sqlite { database, .. } => database,
    };
    let mut registry = SourceRegistry::new(
        id.scope.clone(),
        id.target_host.clone(),
        id.instance_id.clone(),
        id.provider,
        vec![root.clone()],
    )
    .map_err(|e| e.kind())?;
    registry.register(&session, root.join(relative)).map_err(|e| e.kind())?;
    Ok(registry)
}
pub(super) fn app(provider: SessionProvider) -> AppId {
    match provider {
        SessionProvider::Claude => AppId::Claude,
        SessionProvider::Codex => AppId::Codex,
        SessionProvider::Gemini => AppId::Gemini,
        SessionProvider::Opencode => AppId::OpenCode,
        SessionProvider::Openclaw => AppId::OpenClaw,
        SessionProvider::Hermes => AppId::Hermes,
        SessionProvider::Grokbuild => AppId::GrokBuild,
        SessionProvider::Pi => AppId::Pi,
        SessionProvider::Omp => AppId::Omp,
        SessionProvider::Kimi => AppId::Kimi,
        SessionProvider::Mcode => AppId::Mcode,
        SessionProvider::Dsh => AppId::Dsh,
        SessionProvider::Agy => AppId::Agy,
    }
}
