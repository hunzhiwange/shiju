//! Read-only access to authorized native model deployments and built-in presets.

use super::agent_apps::provider_config_home;
use crate::{
    app_providers::AppProviderStore,
    global_database::GlobalDatabase,
    project::{self, instance, store::ProjectStore},
};
use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use vw_agent_apps::models::{
    capabilities::resolve_model_capabilities, codex_template::codex_custom_template,
    user_agent_presets::USER_AGENT_PRESETS,
};
use vw_api_types::{
    agent_apps::AppId,
    common::PaginatedResponse,
    id::{ModelId, ProjectId, ProjectScope, TenantScope},
    model_catalog::{ModelCatalogCapabilities, ModelCatalogIdentity, ModelCatalogTemplates},
};

#[path = "model_catalog/presets.rs"]
mod presets;

#[derive(Clone)]
struct Policy(Option<PathBuf>);

#[derive(Clone, Copy)]
enum View {
    Models,
    Presets,
    Templates,
    Capabilities,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Params {
    project_id: ProjectId,
    directory: String,
    target_host_id: String,
    app_id: AppId,
    provider_id: Option<String>,
    endpoint: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
    model_id: Option<ModelId>,
}

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/model-catalog", get(read).layer(Extension(View::Models)))
        .route("/model-catalog/presets", get(read).layer(Extension(View::Presets)))
        .route("/model-catalog/templates", get(read).layer(Extension(View::Templates)))
        .route("/model-catalog/capabilities", get(read).layer(Extension(View::Capabilities)))
        .layer(Extension(Policy(path)))
}

async fn read(
    Extension(view): Extension<View>,
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<Params>, QueryRejection>,
) -> Result<Response, StatusCode> {
    let Query(params) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        provider_config_home(policy.0.as_deref(), &headers, &tenant, &params.target_host_id)?;
        let (database, projects) = match database {
            Some(Extension(db)) => {
                let projects =
                    ProjectStore::new(db.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                (db, projects)
            }
            None => (
                GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                project::runtime_store().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            ),
        };
        let context = registered_instance(&projects, &tenant, &params)?;
        instance::provide_context_sync(context, || read_view(view, params, &tenant, database))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

fn registered_instance(
    projects: &ProjectStore,
    tenant: &TenantScope,
    params: &Params,
) -> Result<instance::Context, StatusCode> {
    let directory = Path::new(&params.directory);
    if !directory.is_absolute() || directory == Path::new("/") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let scope = ProjectScope::new(tenant.clone(), params.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let project = projects
        .get(&scope)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::FORBIDDEN)?;
    let registered = std::iter::once(&project.worktree)
        .chain(&project.sandboxes)
        .find(|path| Path::new(path) == directory)
        .ok_or(StatusCode::FORBIDDEN)?;

    // Use the verified project snapshot. Directory discovery would enter the default tenant
    // and could register an untrusted path or initialize project services on a read request.
    Ok(instance::Context { directory: registered.clone(), worktree: registered.clone(), project })
}

fn read_view(
    view: View,
    params: Params,
    tenant: &TenantScope,
    database: GlobalDatabase,
) -> Result<Response, StatusCode> {
    let app = params.app_id;
    let search = params.search.as_deref().unwrap_or_default();
    if search.len() > 256 || search.chars().any(char::is_control) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let page = Page::parse(params.limit, params.cursor.as_deref())?;
    match view {
        View::Presets => Ok(Json(page.take(presets::search(app, search))?).into_response()),
        View::Templates => Ok(Json(ModelCatalogTemplates {
            codex_config: (app == AppId::Codex).then(|| codex_custom_template().config),
            user_agents: USER_AGENT_PRESETS.iter().map(|ua| (*ua).into()).collect(),
        })
        .into_response()),
        View::Models | View::Capabilities => {
            let deployment = ModelCatalogIdentity {
                target_host_id: params.target_host_id,
                app_id: app,
                provider_id: params.provider_id.ok_or(StatusCode::BAD_REQUEST)?,
                endpoint: params.endpoint.ok_or(StatusCode::BAD_REQUEST)?,
            };
            vw_agent_apps::providers::validate_identity(&deployment.target_host_id)
                .and_then(|_| vw_agent_apps::providers::validate_identity(&deployment.provider_id))
                .and_then(|_| vw_agent_apps::providers::validation::endpoint(&deployment.endpoint))
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let mut catalog = AppProviderStore::new(database)
                .and_then(|store| store.read_model_catalog(tenant, &deployment))
                .map_err(|err| match err.as_str() {
                    "application provider not found" => StatusCode::NOT_FOUND,
                    _ => StatusCode::SERVICE_UNAVAILABLE,
                })?;
            if matches!(view, View::Capabilities) {
                let id = params.model_id.ok_or(StatusCode::BAD_REQUEST)?;
                let model = catalog
                    .items
                    .into_iter()
                    .find(|row| row.identity.model_id == id)
                    .ok_or(StatusCode::NOT_FOUND)?;
                let capabilities = resolve_model_capabilities(
                    model.identity.model_id.as_ref(),
                    None,
                    &model.metadata,
                    false,
                );
                let (image_input, text_only) = (capabilities.image_input, capabilities.text_only);
                return Ok(Json(ModelCatalogCapabilities { model, image_input, text_only })
                    .into_response());
            }

            if !search.trim().is_empty() {
                let search = search.to_lowercase();
                catalog.items.retain(|row| {
                    let id = row.identity.model_id.as_ref().to_lowercase();
                    let alias = row.display_alias.as_deref().unwrap_or_default().to_lowercase();
                    search.split_whitespace().all(|term| id.contains(term) || alias.contains(term))
                });
            }
            let page = page.take(catalog.items)?;
            catalog.items = page.items;
            catalog.next_cursor = page.next_cursor;
            Ok(Json(catalog).into_response())
        }
    }
}

struct Page {
    offset: usize,
    limit: usize,
}

impl Page {
    fn parse(limit: Option<u32>, cursor: Option<&str>) -> Result<Self, StatusCode> {
        let limit = limit.unwrap_or(50);
        if !(1..=100).contains(&limit) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let offset = match cursor {
            None => 0,
            Some(cursor) if cursor.len() <= 10 && cursor.bytes().all(|b| b.is_ascii_digit()) => {
                cursor.parse().map_err(|_| StatusCode::BAD_REQUEST)?
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        Ok(Self { offset, limit: limit as usize })
    }

    fn take<T>(self, items: Vec<T>) -> Result<PaginatedResponse<T>, StatusCode> {
        if self.offset > items.len() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let end = self.offset.saturating_add(self.limit);
        let next_cursor = (end < items.len()).then(|| end.to_string());
        Ok(PaginatedResponse {
            items: items.into_iter().skip(self.offset).take(self.limit).collect(),
            next_cursor,
        })
    }
}
