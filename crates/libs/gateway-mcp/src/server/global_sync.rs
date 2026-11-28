//! Synchronize a saved global server to native app configuration on an authorized host.
use axum::{
    Extension, Json, Router,
    extract::Query,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware,
    response::Response,
    routing::{get, post},
};
use vw_api_mcp_management::{McpFailureCode, McpPresetDraftView};
use vw_api_transport::TenantScope;
use crate::global_sync::{Options, SyncItem, SyncRequest};

pub fn router() -> Router {
    Router::new()
        .route("/global/mcp/options", get(options))
        .route("/global/mcp/sync", post(sync))
        .layer(middleware::map_response(async |mut response: Response| {
            response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OptionsQuery {
    target_host_id: String,
}

async fn options(
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(query): Query<OptionsQuery>,
) -> Result<Json<Options>, StatusCode> {
    use vw_agent_apps::mcp::presets;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    tokio::task::spawn_blocking(move || {
        super::authorization::authorize_mcp_catalog_read(
            policy.as_deref(), &headers, &tenant, &query.target_host_id,
        )?;
        let presets = presets::list_presets().iter().filter_map(|preset| {
            let draft = presets::create_preset_draft(preset.id, &[]).ok()??;
            Some(McpPresetDraftView {
                id: preset.id.into(),
                name: preset.name.into(),
                tags: preset.tags.iter().map(|tag| (*tag).into()).collect(),
                homepage: preset.homepage.into(),
                documentation: preset.documentation.into(),
                connection: draft.connection,
                enabled: draft.enabled,
            })
        }).collect();
        #[cfg(not(target_os = "macos"))]
        let applications = Vec::new();
        #[cfg(target_os = "macos")]
        let applications = super::authorization::authorize_mcp_native_write(
            policy.as_deref(), &headers, &tenant, &query.target_host_id,
        ).ok().map_or_else(Vec::new, |(host, _)| {
            use vw_agent_apps::{apps::{catalog, discovery::read}, mcp::native_capability};
            catalog::catalog().into_iter()
                .map(|entry| entry.metadata.id)
                .filter(|&app| native_app(app) && read::locate_cli(app, &host).is_ok())
                .map(native_capability)
                .collect()
        });
        Ok(Json(Options { presets, applications }))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn sync(
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    database: Option<Extension<agent_database::GlobalDatabase>>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<Vec<SyncItem>>, StatusCode> {
    use vw_api_mcp_management::McpCatalogScope;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    super::authorization::authorize_mcp_catalog_write(
        policy.as_deref(), &headers, &scope, &request.target_host_id,
    )?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (database, &request.applications, McpFailureCode::TargetUnavailable);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        let (host, root) = super::authorization::authorize_mcp_native_write(
            policy.as_deref(), &headers, &tenant, &request.target_host_id,
        )?;
        let name = vw_agent_apps::mcp::validation::validate_new_name(&request.name, &[])
            .map_err(|_| StatusCode::BAD_REQUEST)?.0;
        let apps = request.applications;
        if apps.is_empty() || apps.len() > 16
            || apps.iter().enumerate().any(|(index, app)| !native_app(*app) || apps[..index].contains(app))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        let config = agent_config::runtime::get_global_for_tenant(&tenant).await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let mut servers = config.mcp_servers.iter().filter(|server| {
            server["name"].as_str().map(str::trim) == Some(name.as_str())
        });
        let server = servers.next().ok_or(StatusCode::NOT_FOUND)?;
        if servers.next().is_some() {
            return Err(StatusCode::CONFLICT);
        }
        let connection = crate::global_sync::connection(server).map_err(|_| StatusCode::BAD_REQUEST)?;
        vw_agent_apps::mcp::validation::validate_connection(&connection)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        tokio::task::spawn_blocking(move || {
            use agent_app_services::mcp::McpCatalogStore;
            use agent_database::GlobalDatabase;
            use app_settings_store::settings_store::GatewaySettingsStore;
            use vw_agent_apps::{
                apps::discovery::read,
                mcp::{distribution::{NativeMcpEdit, NativeMcpWrite}, import::NativeMcpSource},
                providers::live::commit::{ExecutionHostId, ProviderTransactions},
            };
            use vw_api_mcp_management::McpNativeStatus;

            let unavailable = |_| StatusCode::SERVICE_UNAVAILABLE;
            let database = match database {
                Some(Extension(database)) => database,
                None => GlobalDatabase::for_home(&agent_paths::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store = GatewaySettingsStore::new(database.clone()).map_err(unavailable)?;
            // Recognize ownership from an earlier release without copying its definitions.
            let previous = McpCatalogStore::new(database)
                .and_then(|store| store.list_for_host(&scope, &host.host_id))
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.1;
            let host_id = ExecutionHostId::new(&host.host_id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let transactions = ProviderTransactions::new(&root, host_id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let mut results = Vec::with_capacity(apps.len());
            for app in apps {
                let result = (|| {
                    read::locate_cli(app, &host).map_err(|_| McpFailureCode::TargetUnavailable)?;
                    let key = serde_json::to_string(&(&host.host_id, app, &name))
                        .map_err(|_| McpFailureCode::InvalidDefinition)?;
                    let namespace = "global-mcp-sync";
                    let owned = store.get(&tenant, namespace, &key)
                        .map_err(|_| McpFailureCode::StorageUnavailable)?.is_some()
                        || previous.iter().any(|binding| {
                            binding.record.id.0 == name && binding.target.app_id == app
                                && binding.native_status == McpNativeStatus::Present
                        });
                    let writer = NativeMcpWrite {
                        tenant: &tenant,
                        app_id: app,
                        target_host_id: &host.host_id,
                        source: NativeMcpSource::HostHome(&host.home),
                        transactions: &transactions,
                    };
                    let edit = if owned { NativeMcpEdit::Update(&connection) } else { NativeMcpEdit::Insert(&connection) };
                    match writer.apply(&name, edit, || Ok(())) {
                        Err(McpFailureCode::Conflict) if owned => {
                            writer.apply(&name, NativeMcpEdit::Insert(&connection), || Ok(()))?;
                        }
                        result => { result?; }
                    }
                    store.upsert(&tenant, namespace, &key, "true")
                        .map_err(|_| McpFailureCode::StorageUnavailable)
                })();
                results.push(SyncItem { app_id: app, failure: result.err() });
            }
            Ok(Json(results))
        }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}

#[cfg(target_os = "macos")]
fn native_app(app: vw_api_app_identity::AppId) -> bool {
    use vw_api_app_identity::AppId::*;
    !matches!(app, Claude | ClaudeDesktop | OpenClaw)
}
