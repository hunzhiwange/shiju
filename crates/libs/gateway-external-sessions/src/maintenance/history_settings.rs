//! Settings publication is separate from explicit maintenance submission.
use super::*;

pub(in super::super) async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(target): Json<HistoryMaintenanceTarget>,
) -> Result<Json<HistoryOptions>, StatusCode> {
    if target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, headers);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            super::host::authorize(&policy, &headers, &target)?;
            super::host::options(&target).map(Json)
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}

pub(in super::super) async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<HistoryOptionsUpdate>,
) -> Result<Json<HistoryOptions>, StatusCode> {
    if request.target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, headers);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            super::host::authorize(&policy, &headers, &request.target)?;
            super::host::set_options(&request.target, &request.options)?;
            Ok(Json(request.options))
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}

pub(in super::super) async fn backups(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(target): Json<HistoryMaintenanceTarget>,
) -> Result<Json<HistoryBackups>, StatusCode> {
    if target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, headers);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            let (_, _, root, private) = super::host::authorize(&policy, &headers, &target)?;
            super::host::backups(&target, &root, &private).map(Json)
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}
