//! Reads persisted identities only after project/host authorization.
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::get};
use vw_api_types::{agent_apps_skills::{inventory_api::*, repos_api::SkillReposQuery}, id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

#[derive(Clone)]
struct InventoryPolicy(Option<std::path::PathBuf>);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/skills", get(list))
        .route("/agent-apps/skills/{id}", get(detail))
        .layer(Extension(InventoryPolicy(policy)))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn list(Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<InventoryPolicy>, headers: HeaderMap, Query(scope): Query<SkillReposQuery>,
) -> Result<Json<SkillInventoryResponse>, StatusCode> {
    read(tenant, database, policy, headers, scope, None).await.map(Json)
}

async fn detail(Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<InventoryPolicy>, headers: HeaderMap, Query(scope): Query<SkillReposQuery>, Path(id): Path<String>,
) -> Result<Json<InstalledSkill>, StatusCode> {
    let response = read(tenant, database, policy, headers, scope, Some(id)).await?;
    response.skills.into_iter().next().map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn read(tenant: TenantScope, database: Option<Extension<GlobalDatabase>>, policy: InventoryPolicy, headers: HeaderMap,
    scope: SkillReposQuery, id: Option<String>,
) -> Result<SkillInventoryResponse, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let root = super::super::agent_apps::authorize_skill_inventory(policy.0.as_deref(), &headers,
            &project, &scope.target_host_id)?;
        #[cfg(target_os = "macos")]
        {
            use crate::gateway::agent_apps_skills_store::SkillInstallsStore;
            use vw_agent_apps::skills::{inventory::get_all_installed, local_install::validate_directory};
            if let Some(id) = &id { validate_directory(id).map_err(|_| StatusCode::BAD_REQUEST)?; }
            let db = match database {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store = SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let mut state = store.get(&project, &scope.target_host_id, &root)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            if let Some(id) = id {
                state.skills.retain(|skill| skill.directory == id);
                if state.skills.is_empty() { return Err(StatusCode::NOT_FOUND); }
            }
            let skills = get_all_installed(&state.ssot_root, &state.skills)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(SkillInventoryResponse { scope, version: state.version, skills: skills.into_iter().map(|detail| InstalledSkill {
                source: state.repo_sources.get(&detail.skill.directory).map(|source|
                    format!("{}/{} · {} · {}", source.owner, source.name, source.branch, source.directory)),
                id: detail.skill.directory, name: detail.skill.name, description: detail.skill.description,
                present: detail.present, content_hash: detail.content_hash, version: detail.version,
                markdown: detail.markdown, manifest: detail.manifest,
                resources: detail.resources.into_iter().map(|path| path.to_string_lossy().into_owned()).collect(),
            }).collect() })
        }
        #[cfg(not(target_os = "macos"))]
        { let _ = (database, root, id); Err(StatusCode::NOT_IMPLEMENTED) }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
