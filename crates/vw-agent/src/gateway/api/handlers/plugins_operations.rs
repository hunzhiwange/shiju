//! Authenticated, configuration-bound package previews and explicit activation.
use super::*;
use crate::gateway::AppState;
use axum::{extract::State, http::HeaderMap};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use vw_gateway_client::DesktopPluginPreviewDto;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/plugins/prepare-install", post(prepare_install))
        .route("/plugins/install", post(install))
        .route("/plugins/prepare-update", post(prepare_update))
        .route("/plugins/update", post(update))
        .route("/plugins/cancel", post(cancel))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRequest {
    source: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateRequest {
    plugin_id: String,
    source: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationRequest {
    token: String,
    approved: bool,
}
struct PreparedInstall {
    installed: crate::plugins::PluginInstallResult,
    hash: String,
    _owner: tempfile::TempDir,
}
enum Operation {
    Install(PreparedInstall),
    Update(crate::plugins::PluginUpgradePreview),
}
struct Pending {
    root: PathBuf,
    created: Instant,
    operation: Operation,
}
fn pending() -> &'static Mutex<HashMap<String, Pending>> {
    static PENDING: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}
fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    super::super::super::auth::require_auth(state, headers)
        .map_err(|(status, _)| ApiError { status, message: "Unauthorized".into() })
}
fn bad(error: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(error.to_string())
}
fn store(root: PathBuf, operation: Operation) -> Result<String> {
    let mut map =
        pending().lock().map_err(|_| anyhow::anyhow!("plugin operation store unavailable"))?;
    map.retain(|_, p| p.created.elapsed() < Duration::from_secs(600));
    if map.len() >= 16 {
        bail!("too many pending plugin previews");
    }
    let token = uuid::Uuid::new_v4().to_string();
    map.insert(token.clone(), Pending { root, created: Instant::now(), operation });
    Ok(token)
}
fn validate_source(source: &str) -> Result<()> {
    let source = source.trim();
    if source.trim().is_empty() || source.len() > 4096 {
        bail!("invalid plugin source");
    }
    if source.starts_with("http://") || source.starts_with("https://") {
        bail!("use an audited git+, npm:, local package or registered marketplace source");
    }
    Ok(())
}
fn origin(manifest: &PluginManifest) -> Result<vw_gateway_client::DesktopPluginSourceDto> {
    let metadata = crate::plugins::plugin_install_metadata(&manifest.root, manifest)?
        .ok_or_else(|| anyhow::anyhow!("prepared plugin provenance is missing"))?;
    let value: serde_json::Value = serde_json::from_str(&metadata.source).unwrap_or_default();
    let field = |key| value.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
    Ok(vw_gateway_client::DesktopPluginSourceDto {
        kind: if value.get("npm").is_some() {
            "npm"
        } else if value.get("git").is_some() {
            "git"
        } else {
            "local"
        }
        .into(),
        resolved_version: field("version").or(metadata.version),
        revision: field("commit"),
        sha256: field("sha256"),
    })
}
fn preview(
    token: String,
    source: String,
    installed: &crate::plugins::PluginInstallResult,
    old_version: Option<String>,
) -> Result<DesktopPluginPreviewDto> {
    let dependency_checks = super::dependency_checks(&installed.manifest);
    Ok(DesktopPluginPreviewDto {
        token,
        plugin_id: installed.identity.as_key(),
        source,
        old_version,
        version: installed.manifest.version.clone(),
        capabilities: super::plugin_capabilities(&installed.manifest),
        audit_status: "passed".into(),
        dependency_status: super::dependency_status(&dependency_checks),
        dependency_checks,
        origin: origin(&installed.manifest)?,
    })
}
async fn prepare_install(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SourceRequest>,
) -> Result<Json<DesktopPluginPreviewDto>, ApiError> {
    authorize(&state, &headers)?;
    let config = state.config.lock().clone();
    tokio::task::spawn_blocking(move || {
        validate_source(&body.source)?;
        let temporary = tempfile::tempdir()?;
        let destination = temporary.path().join("plugins");
        let installed = if !Path::new(&body.source).exists()
            && crate::plugins::PluginIdentity::parse_marketplace(&body.source).is_ok()
        {
            let entry = crate::plugins::registered_plugin_entry(
                &super::config_dir_from_config(&config),
                &body.source,
            )?;
            crate::plugins::install_plugin_marketplace_entry(&entry, &destination)?
        } else {
            crate::plugins::install_plugin_from_source(&body.source, &destination)?
        };
        let hash =
            crate::plugins::plugin_package_content_hash(&installed.installed_dir, &|| false)?;
        let response = preview(String::new(), body.source, &installed, None)?;
        let token = store(
            install_dir_from_config(&config),
            Operation::Install(PreparedInstall { installed, hash, _owner: temporary }),
        )?;
        Ok::<_, anyhow::Error>(DesktopPluginPreviewDto { token, ..response })
    })
    .await
    .map_err(|_| ApiError::internal("plugin preparation task failed"))?
    .map(Json)
    .map_err(bad)
}
async fn prepare_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateRequest>,
) -> Result<Json<DesktopPluginPreviewDto>, ApiError> {
    authorize(&state, &headers)?;
    let config = state.config.lock().clone();
    tokio::task::spawn_blocking(move || {
        let root = install_dir_from_config(&config);
        let source = body.source.clone().unwrap_or_else(|| body.plugin_id.clone());
        validate_source(&source)?;
        let prepared = if let Some(source) = body.source {
            crate::plugins::prepare_plugin_upgrade(&source, &root, &body.plugin_id)?
        } else {
            let entry = crate::plugins::registered_plugin_entry(
                &super::config_dir_from_config(&config),
                &body.plugin_id,
            )?;
            crate::plugins::prepare_plugin_marketplace_upgrade(&entry, &root)?
        };
        let capabilities = super::plugin_capabilities(prepared.manifest());
        let dependency_checks = super::dependency_checks(prepared.manifest());
        let response = DesktopPluginPreviewDto {
            token: String::new(),
            plugin_id: body.plugin_id,
            source,
            old_version: prepared.old_version.clone(),
            version: prepared.new_version.clone(),
            capabilities,
            audit_status: "passed".into(),
            dependency_status: super::dependency_status(&dependency_checks),
            dependency_checks,
            origin: origin(prepared.manifest())?,
        };
        let token = store(root, Operation::Update(prepared))?;
        Ok::<_, anyhow::Error>(DesktopPluginPreviewDto { token, ..response })
    })
    .await
    .map_err(|_| ApiError::internal("plugin upgrade preparation task failed"))?
    .map(Json)
    .map_err(bad)
}
async fn install(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ActivationRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    activate(state, headers, body, false).await
}
async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ActivationRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    activate(state, headers, body, true).await
}
async fn activate(
    state: AppState,
    headers: HeaderMap,
    body: ActivationRequest,
    is_update: bool,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    authorize(&state, &headers)?;
    if !body.approved {
        return Err(ApiError::bad_request("explicit approval is required"));
    }
    let config = state.config.lock().clone();
    tokio::task::spawn_blocking(move || {
        let root = install_dir_from_config(&config);
        let operation = {
            let mut map = pending()
                .lock()
                .map_err(|_| anyhow::anyhow!("plugin operation store unavailable"))?;
            map.retain(|_, p| p.created.elapsed() < Duration::from_secs(600));
            let p = map
                .get(&body.token)
                .ok_or_else(|| anyhow::anyhow!("unknown or expired plugin preview"))?;
            if p.root != root || matches!(&p.operation, Operation::Update(_)) != is_update {
                bail!("plugin preview belongs to another scope or operation");
            }
            map.remove(&body.token)
                .ok_or_else(|| anyhow::anyhow!("plugin preview already consumed"))?
                .operation
        };
        let path = match operation {
            Operation::Install(p) => {
                if crate::plugins::plugin_package_content_hash(&p.installed.installed_dir, &|| {
                    false
                })? != p.hash
                {
                    bail!("prepared plugin contents changed");
                }
                crate::plugins::publish_materialized_install(&p.installed, &root)?.installed_dir
            }
            Operation::Update(p) => {
                let key = p.identity_key();
                crate::plugins::activate_plugin_upgrade(p)?;
                root.join(key)
            }
        };
        Ok::<_, anyhow::Error>(DesktopPluginPathDto { path: path.to_string_lossy().into_owned() })
    })
    .await
    .map_err(|_| ApiError::internal("plugin activation task failed"))?
    .map(Json)
    .map_err(bad)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelRequest {
    token: String,
}
async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authorize(&state, &headers)?;
    let root = install_dir_from_config(&state.config.lock());
    tokio::task::spawn_blocking(move || {
        let removed = {
            let mut map = pending()
                .lock()
                .map_err(|_| anyhow::anyhow!("plugin operation store unavailable"))?;
            if map.get(&body.token).is_some_and(|p| p.root != root) {
                bail!("plugin preview belongs to another scope");
            }
            map.remove(&body.token)
        };
        drop(removed);
        Ok::<_, anyhow::Error>(serde_json::json!({"cancelled":true}))
    })
    .await
    .map_err(|_| ApiError::internal("plugin cancellation task failed"))?
    .map(Json)
    .map_err(bad)
}
