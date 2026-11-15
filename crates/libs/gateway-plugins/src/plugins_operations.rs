//! Authenticated, configuration-bound package previews and explicit activation.
use super::*;

use axum::{extract::State, http::HeaderMap};
use std::time::{Duration, Instant};
use gateway_desktop_settings::DesktopPluginPreviewDto;

pub(super) fn router<H: PluginHost>() -> Router<H::State> {
    Router::new()
        .route("/plugins/status", get(status::<H>))
        .route("/plugins/prepare-install", post(prepare_install::<H>))
        .route("/plugins/install", post(install::<H>))
        .route("/plugins/prepare-update", post(prepare_update::<H>))
        .route("/plugins/update", post(update::<H>))
        .route("/plugins/cancel", post(cancel::<H>))
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
    installed: agent_plugins::PluginInstallResult,
    hash: String,
    _owner: tempfile::TempDir,
}
enum Operation<U> {
    Install(PreparedInstall),
    Update(U),
}
pub struct Pending<U> {
    root: PathBuf,
    created: Instant,
    operation: Operation<U>,
}
fn authorize<H: PluginHost>(state: &H::State, headers: &HeaderMap) -> Result<(), ApiError> {
    H::authorize(state, headers)
}
fn bad(error: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(error.to_string())
}
fn store<H: PluginHost>(root: PathBuf, operation: Operation<H::Upgrade>) -> Result<String> {
    let mut map =
        H::pending().lock().map_err(|_| anyhow::anyhow!("plugin operation store unavailable"))?;
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
fn origin(manifest: &PluginManifest) -> Result<gateway_desktop_settings::DesktopPluginSourceDto> {
    let metadata = plugin_install::identity::read_install_metadata(&manifest.root, manifest)?
        .ok_or_else(|| anyhow::anyhow!("prepared plugin provenance is missing"))?;
    Ok(super::source_origin(Some(&metadata), manifest.version.clone()))
}
async fn status<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
) -> Result<Json<Vec<DesktopPluginCatalogEntryDto>>, ApiError> {
    authorize::<H>(&state, &headers)?;
    let config = H::state_config(&state);
    tokio::task::spawn_blocking(move || {
        super::collect_plugin_catalog::<H>(
            config.workspace_dir.clone(),
            super::config_dir_from_config(&config),
            config.plugins.clone(),
        )
    })
    .await
    .map_err(|_| ApiError::internal("plugin status task failed"))?
    .map(Json)
    .map_err(bad)
}
fn preview<H: PluginHost>(
    token: String,
    source: String,
    installed: &agent_plugins::PluginInstallResult,
    old_version: Option<String>,
) -> Result<DesktopPluginPreviewDto> {
    let dependency_checks = super::dependency_checks::<H>(&installed.manifest);
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
        previous_capabilities: None,
    })
}
async fn prepare_install<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
    Json(body): Json<SourceRequest>,
) -> Result<Json<DesktopPluginPreviewDto>, ApiError> {
    authorize::<H>(&state, &headers)?;
    let config = H::state_config(&state);
    tokio::task::spawn_blocking(move || {
        validate_source(&body.source)?;
        let temporary = tempfile::tempdir()?;
        let destination = temporary.path().join("plugins");
        let installed = if !Path::new(&body.source).exists()
            && agent_plugins::PluginIdentity::parse_marketplace(&body.source).is_ok()
        {
            let entry = plugin_install::marketplace::registered_plugin_entry(
                &super::config_dir_from_config(&config),
                &body.source,
            )?;
            H::install_plugin_marketplace_entry(&entry, &destination)?
        } else {
            H::install_plugin_from_source(&body.source, &destination)?
        };
        let hash =
            plugin_hooks::hook_trust_content::plugin_content_hash(&installed.installed_dir, &|| false)?;
        let response = preview::<H>(String::new(), body.source, &installed, None)?;
        let token = store::<H>(
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
async fn prepare_update<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
    Json(body): Json<UpdateRequest>,
) -> Result<Json<DesktopPluginPreviewDto>, ApiError> {
    authorize::<H>(&state, &headers)?;
    let config = H::state_config(&state);
    tokio::task::spawn_blocking(move || {
        let root = install_dir_from_config(&config);
        let source = body.source.clone().unwrap_or_else(|| body.plugin_id.clone());
        validate_source(&source)?;
        let prepared = if let Some(source) = body.source {
            H::prepare_plugin_upgrade(&source, &root, &body.plugin_id)?
        } else {
            let entry = plugin_install::marketplace::registered_plugin_entry(
                &super::config_dir_from_config(&config),
                &body.plugin_id,
            )?;
            H::prepare_plugin_marketplace_upgrade(&entry, &root)?
        };
        let capabilities = super::plugin_capabilities(H::manifest(&prepared));
        let dependency_checks = super::dependency_checks::<H>(H::manifest(&prepared));
        let response = DesktopPluginPreviewDto {
            token: String::new(),
            plugin_id: body.plugin_id,
            source,
            old_version: H::old_manifest(&prepared).version.clone(),
            version: H::manifest(&prepared).version.clone(),
            capabilities,
            audit_status: "passed".into(),
            dependency_status: super::dependency_status(&dependency_checks),
            dependency_checks,
            origin: origin(H::manifest(&prepared))?,
            previous_capabilities: Some(super::plugin_capabilities(H::old_manifest(&prepared))),
        };
        let token = store::<H>(root, Operation::Update(prepared))?;
        Ok::<_, anyhow::Error>(DesktopPluginPreviewDto { token, ..response })
    })
    .await
    .map_err(|_| ApiError::internal("plugin upgrade preparation task failed"))?
    .map(Json)
    .map_err(bad)
}
async fn install<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
    Json(body): Json<ActivationRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    activate::<H>(state, headers, body, false).await
}
async fn update<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
    Json(body): Json<ActivationRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    activate::<H>(state, headers, body, true).await
}
async fn activate<H: PluginHost>(
    state: H::State,
    headers: HeaderMap,
    body: ActivationRequest,
    is_update: bool,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    authorize::<H>(&state, &headers)?;
    if !body.approved {
        return Err(ApiError::bad_request("explicit approval is required"));
    }
    let config = H::state_config(&state);
    tokio::task::spawn_blocking(move || {
        let root = install_dir_from_config(&config);
        let operation = {
            let mut map = H::pending()
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
                if plugin_hooks::hook_trust_content::plugin_content_hash(&p.installed.installed_dir, &|| {
                    false
                })? != p.hash
                {
                    bail!("prepared plugin contents changed");
                }
                agent_plugins::install::publish_materialized_install(&p.installed, &root)?.installed_dir
            }
            Operation::Update(p) => {
                let key = H::identity_key(&p);
                H::activate_plugin_upgrade(p)?;
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
async fn cancel<H: PluginHost>(
    State(state): State<H::State>,
    headers: HeaderMap,
    Json(body): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authorize::<H>(&state, &headers)?;
    let root = install_dir_from_config(&H::state_config(&state));
    tokio::task::spawn_blocking(move || {
        let removed = {
            let mut map = H::pending()
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
