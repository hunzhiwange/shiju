//! Authenticated desktop management of the gateway's single local control host.
use axum::{Extension, Json, Router, extract::DefaultBodyLimit, http::{HeaderMap, StatusCode}, middleware, response::{IntoResponse, Response}, routing::{get, post}};
use serde_json::json;
use sha2::{Digest, Sha256};
use vw_api_types::{computer_control::*, id::TenantScope};
use vw_config_types::computer_control::GrantDecision;
use crate::{computer_control::{policy::{ComputerControlPolicy, ControlFeature, GrantScope}, office::sessions::OfficeSurface}, tools::computer_control::NativeControlRuntime};

static EDIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
type Failure = StatusCode;
fn user() -> String {
    // SAFETY: geteuid has no preconditions.
    format!("uid:{}", unsafe { libc::geteuid() })
}
pub(crate) fn router() -> Router {
    Router::new()
        .route("/computer-control/status", get(status))
        .route("/computer-control/settings", axum::routing::put(save))
        .route("/computer-control/approvals", get(approvals).delete(revoke))
        .route("/computer-control/pairing", post(pair))
        .route("/computer-control/stop", post(stop))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::map_response(async |mut response: Response| {
            response.headers_mut().insert("cache-control", axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
async fn runtime(tenant: &TenantScope, headers: &HeaderMap) -> Result<std::sync::Arc<NativeControlRuntime>, Failure> {
    if headers.get_all("authorization").iter().count() != 1 { return Err(StatusCode::UNAUTHORIZED); }
    let token = crate::gateway::api::auth::extract_auth_skey(headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let config = crate::config::get_global_for_tenant(tenant).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let guard = crate::security::pairing::PairingGuard::from_skeys(config.gateway.auth_enabled, &config.gateway.skeys);
    if !guard.auth_enabled() || !guard.is_authenticated(token) { return Err(StatusCode::UNAUTHORIZED); }
    // Bootstrap owns a default-tenant host; another tenant's credential cannot manage it.
    if tenant != &TenantScope::default() { return Err(StatusCode::FORBIDDEN); }
    NativeControlRuntime::shared().ok_or(StatusCode::SERVICE_UNAVAILABLE)
}
fn feature(enabled: bool, readiness: ControlReadiness, authorization: SystemAuthorization, backend: ControlBackend, capabilities: Vec<ControlCapability>) -> ControlFeatureStatus {
    ControlFeatureStatus { enabled, readiness: if enabled { readiness } else { ControlReadiness::Disabled }, system_authorization: authorization, connection: ControlConnection::Disconnected, backend, capabilities }
}
async fn status(Extension(t): Extension<TenantScope>, h: HeaderMap) -> Result<Json<ComputerControlStatus>, Failure> {
    let runtime = runtime(&t, &h).await?;
    let chrome_connected = runtime.chrome_connected().await;
    let documents = runtime.office_transport().map(|o| o.connected_documents()).unwrap_or_default();
    let policy = runtime.control_policy().read();
    let config = policy.config().users.get(&user()).cloned().unwrap_or_default();
    let permissions = crate::computer_control::native::native_permissions();
    let (readiness, auth, backend, caps) = match permissions {
        Ok(p) => (if p.accessibility && p.screen_recording { ControlReadiness::Ready } else { ControlReadiness::Unauthorized }, if p.accessibility && p.screen_recording { SystemAuthorization::Granted } else { SystemAuthorization::Denied }, ControlBackend::Available, vec![ControlCapability::Observe, ControlCapability::Screenshot, ControlCapability::Input]),
        Err(_) => (ControlReadiness::Unsupported, SystemAuthorization::NotChecked, ControlBackend::Unsupported, vec![]),
    };
    let bridge = |enabled, connected, capabilities| {
        let mut status = feature(enabled, if connected { ControlReadiness::Ready } else { ControlReadiness::Disconnected }, SystemAuthorization::NotRequired, ControlBackend::Available, capabilities);
        status.connection = if connected { ControlConnection::Connected } else { ControlConnection::Disconnected };
        status
    };
    let office = |enabled, surface| {
        use crate::computer_control::office::sessions::OfficeCapability as C;
        let mut capabilities = Vec::new();
        let sessions: Vec<_> = documents.iter().filter(|d| d.handle.surface == surface).collect();
        for session in &sessions {
            for capability in &session.capabilities {
                let mapped = match capability { C::ExcelReadRange | C::WordReadDocument | C::WordFindText => ControlCapability::StructuredRead, _ => ControlCapability::StructuredWrite };
                if !capabilities.contains(&mapped) { capabilities.push(mapped); }
            }
        }
        bridge(enabled, !sessions.is_empty(), capabilities)
    };
    Ok(Json(ComputerControlStatus {
        permission_version: policy.version(),
        native_apps: feature(config.native_apps, readiness, auth, backend, caps),
        chrome: bridge(config.chrome, chrome_connected, vec![]), excel: office(config.excel, OfficeSurface::Excel), word: office(config.word, OfficeSurface::Word),
        locked_use: LockedUseStatus { status: feature(config.locked_use, ControlReadiness::Unsupported, SystemAuthorization::NotChecked, ControlBackend::Unsupported, vec![]), probe: LockedUseProbe::Unsupported },
    }))
}
async fn approvals(Extension(t): Extension<TenantScope>, h: HeaderMap) -> Result<Json<ControlApprovals>, Failure> {
    let runtime = runtime(&t, &h).await?;
    let policy = runtime.control_policy().read();
    let mut approvals = Vec::new();
    if let Some(c) = policy.config().users.get(&user()) {
        for (map, constructor) in [(&c.applications, ControlGrantScope::Application as fn(String) -> ControlGrantScope), (&c.chrome_origins, ControlGrantScope::ChromeOrigin), (&c.excel_documents, ControlGrantScope::ExcelDocument), (&c.word_documents, ControlGrantScope::WordDocument)] {
            approvals.extend(map.iter().filter(|(_, d)| **d == GrantDecision::Allow).map(|(id, _)| constructor(id.clone())));
        }
    }
    Ok(Json(ControlApprovals { permission_version: policy.version(), approvals }))
}
fn setting(f: ControlSetting) -> ControlFeature {
    match f { ControlSetting::NativeApps => ControlFeature::NativeApps, ControlSetting::Chrome => ControlFeature::Chrome, ControlSetting::Excel => ControlFeature::Excel, ControlSetting::Word => ControlFeature::Word, ControlSetting::LockedUse => ControlFeature::LockedUse }
}
fn scope(s: ControlGrantScope) -> GrantScope {
    match s { ControlGrantScope::Application(id) => GrantScope::Application(id), ControlGrantScope::ChromeOrigin(id) => GrantScope::ChromeOrigin(id), ControlGrantScope::ExcelDocument(id) => GrantScope::ExcelDocument(id), ControlGrantScope::WordDocument(id) => GrantScope::WordDocument(id) }
}
async fn publish(t: &TenantScope, runtime: &NativeControlRuntime, next: ComputerControlPolicy) -> Result<Json<ControlVersionResponse>, Failure> {
    crate::config::update_global_for_tenant(t, json!({"computer_control":next.config()})).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let version = next.version();
    *runtime.control_policy().write() = next;
    runtime.stop_all().await;
    Ok(Json(ControlVersionResponse { permission_version: version }))
}
async fn save(Extension(t): Extension<TenantScope>, h: HeaderMap, Json(q): Json<ControlSettingsUpdate>) -> Result<Json<ControlVersionResponse>, Failure> {
    let runtime = runtime(&t, &h).await?;
    let _edit = EDIT.lock().await;
    let mut next = ComputerControlPolicy::new(runtime.control_policy().read().config().clone());
    if next.version() != q.permission_version { return Err(StatusCode::CONFLICT); }
    if q.feature == ControlSetting::LockedUse && q.enabled { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
    next.set_enabled(&user(), setting(q.feature), q.enabled).map_err(|_| StatusCode::BAD_REQUEST)?;
    publish(&t, &runtime, next).await
}
async fn revoke(Extension(t): Extension<TenantScope>, h: HeaderMap, Json(q): Json<ControlGrantRevocation>) -> Result<Json<ControlVersionResponse>, Failure> {
    let runtime = runtime(&t, &h).await?;
    let _edit = EDIT.lock().await;
    let mut next = ComputerControlPolicy::new(runtime.control_policy().read().config().clone());
    if next.version() != q.permission_version { return Err(StatusCode::CONFLICT); }
    // Denial also overrides the broad native-app switch; the allow list drops this entry.
    next.set_grant(&user(), scope(q.scope), Some(GrantDecision::Deny)).map_err(|_| StatusCode::BAD_REQUEST)?;
    publish(&t, &runtime, next).await
}
async fn stop(Extension(t): Extension<TenantScope>, h: HeaderMap) -> Result<StatusCode, Failure> {
    let runtime = runtime(&t, &h).await?;
    let _edit = EDIT.lock().await;
    runtime.stop_all().await;
    Ok(StatusCode::NO_CONTENT)
}
async fn pair(Extension(t): Extension<TenantScope>, h: HeaderMap, Json(q): Json<ControlPairingConfirmation>) -> Result<Response, Failure> {
    let runtime = runtime(&t, &h).await?;
    let _edit = EDIT.lock().await;
    if q.session_id.0.trim().is_empty() || q.session_id.0.len() > 256 { return Err(StatusCode::BAD_REQUEST); }
    let surface = match q.feature { ControlSetting::Excel => OfficeSurface::Excel, ControlSetting::Word => OfficeSurface::Word, _ => return Err(StatusCode::UNPROCESSABLE_ENTITY) };
    let transport = runtime.office_transport().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let mut next = ComputerControlPolicy::new(runtime.control_policy().read().config().clone());
    if next.version() != q.permission_version { return Err(StatusCode::CONFLICT); }
    let enabled = next.config().users.get(&user()).is_some_and(|c| match surface { OfficeSurface::Excel => c.excel, OfficeSurface::Word => c.word });
    if !enabled { return Err(StatusCode::FORBIDDEN); }
    let grant = match surface { OfficeSurface::Excel => GrantScope::ExcelDocument(q.document_id.0.clone()), OfficeSurface::Word => GrantScope::WordDocument(q.document_id.0.clone()) };
    next.set_grant(&user(), grant, Some(GrantDecision::Allow)).map_err(|_| StatusCode::BAD_REQUEST)?;
    let identity = serde_json::to_vec(&(&t, &q.session_id.0, &q.workspace_root)).map_err(|_| StatusCode::BAD_REQUEST)?;
    let owner = ControlSessionId(hex::encode(Sha256::digest(identity)));
    let Json(version) = publish(&t, &runtime, next).await?;
    let credential = transport.approve_pairing(owner, surface, q.document_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(ControlPairingResponse { credential, permission_version: version.permission_version }).into_response())
}
