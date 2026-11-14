//! Compression is native host configuration, guarded by explicit maintenance authority.
use super::catalog::Policy;
use axum::{Extension, Json, http::{HeaderMap, StatusCode}};
use vw_api_types::{TenantScope, external_session::*};

pub(super) async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(target): Json<HistoryMaintenanceTarget>,
) -> Result<Json<SessionCompressionState>, StatusCode> {
    run(policy, tenant, headers, target, None).await.map(Json)
}

pub(super) async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<SessionCompressionUpdate>,
) -> Result<Json<SessionCompressionState>, StatusCode> {
    run(policy, tenant, headers, request.target, Some(request.enabled)).await.map(Json)
}

async fn run(
    policy: Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    target: HistoryMaintenanceTarget,
    enabled: Option<bool>,
) -> Result<SessionCompressionState, StatusCode> {
    if target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, headers, target, enabled);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        use vw_agent_apps::apps::lifecycle::coordinator::ToolLifecycleCoordinator;
        use vw_api_types::agent_apps::AppId;
        // Reject unauthorized requests before attempting to reserve the host.
        super::maintenance::host::authorize(&policy, &headers, &target)?;
        ToolLifecycleCoordinator.run(vec![AppId::Codex], move || {
            Ok(execute(&policy, &headers, &target, enabled))
        })
        .await
        .map_err(|_| StatusCode::CONFLICT)?
    }
}

#[cfg(target_os = "macos")]
fn execute(
    policy: &Policy,
    headers: &HeaderMap,
    target: &HistoryMaintenanceTarget,
    requested: Option<bool>,
) -> Result<SessionCompressionState, StatusCode> {
    use vw_agent_apps::{
        providers::live::{
            commit::{ExecutionHostId, ProviderTransactions},
            write_guard::WriteError,
        },
        sessions::compression::CodexCompression,
    };
    let authorize = || super::maintenance::host::authorize(policy, headers, target);
    let binding = authorize()?;
    let (_, home, root, private) = &binding;
    // CodexCompression owns the non-reentrant AppWriteGuard. Never lock it here.
    let mut compression = CodexCompression::open(home, root).map_err(failure)?;
    let validate = || {
        if authorize().map_err(|_| WriteError::Conflict)? != binding {
            return Err(WriteError::Conflict);
        }
        Ok(())
    };
    if let Some(enabled) = requested {
        let tx = ProviderTransactions::open_existing(
            private,
            ExecutionHostId::new(&target.target_host).map_err(failure)?,
        )
        .map_err(failure)?
        .ok_or(StatusCode::CONFLICT)?;
        compression.set_enabled(&tx, &target.scope, enabled, &validate).map_err(failure)?;
    }
    let enabled = compression.is_enabled().map_err(failure)?;
    let usage = compression.disk_usage().map_err(failure)?;
    validate().map_err(failure)?;
    Ok(SessionCompressionState {
        enabled,
        active_bytes: usage.active_bytes,
        archived_bytes: usage.archived_bytes,
    })
}

#[cfg(target_os = "macos")]
fn failure(error: vw_agent_apps::providers::live::write_guard::WriteError) -> StatusCode {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict | WriteError::Recovered => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        WriteError::Io => StatusCode::SERVICE_UNAVAILABLE,
    }
}
