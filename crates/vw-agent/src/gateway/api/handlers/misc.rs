//! 杂项路由模块

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::agent::agent;
use crate::app::agent::command;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::{InstanceQuery, resolve_directory, with_instance};
use crate::app::agent::skill;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/doc", get(not_implemented))
        .route("/command", get(command_list))
        .route("/agent", get(agent_list))
        .route("/agent/{key}/copy", post(agent_copy))
        .route("/skill", get(skill_list))
}

pub(crate) async fn not_implemented() -> Result<Json<Value>, ApiError> {
    Err(ApiError { status: StatusCode::NOT_IMPLEMENTED, message: "not implemented".to_string() })
}

async fn command_list(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<command::command::Info>>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result =
        with_instance(dir, move || Box::pin(async move { Ok(command::command::list().await) }))
            .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct AgentListQuery {
    directory: Option<String>,
    #[serde(default)]
    include_disabled: bool,
}

#[axum::debug_handler]
async fn agent_list(
    Query(query): Query<AgentListQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent::Info>>, ApiError> {
    let dir = resolve_directory(&InstanceQuery { directory: query.directory }, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let mut agents = if query.include_disabled {
                let config = crate::config::get_current()
                    .await
                    .map_err(|error| ApiError::bad_request(error.to_string()))?;
                agent::catalog::merged_agent_catalog(&config.agents)
                    .into_iter()
                    .map(|(key, definition)| agent::Info { key, definition })
                    .collect::<Vec<_>>()
            } else {
                agent::list().await
            };
            for agent in &mut agents {
                agent.definition.api_key = None;
                agent.definition.options.clear();
            }
            if query.include_disabled {
                agents.sort_by(|a, b| {
                    (b.key == "main").cmp(&(a.key == "main")).then_with(|| a.key.cmp(&b.key))
                });
            }
            Ok(agents)
        })
    })
    .await?;
    Ok(Json(result))
}

async fn agent_copy(
    Extension(tenant): Extension<vw_api_types::id::TenantScope>,
    Path(key): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<agent::Info>, ApiError> {
    let explicit = query.directory.as_deref().is_some_and(|dir| !dir.trim().is_empty())
        || headers
            .get("x-vibewindow-directory")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|dir| !dir.trim().is_empty());
    let directory = std::path::PathBuf::from(resolve_directory(&query, &headers));
    let config = if explicit {
        crate::config::get_project_for_tenant(&tenant, &directory).await
    } else {
        crate::config::get_global_for_tenant(&tenant).await
    }
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let mut definition = agent::catalog::merged_agent_catalog(&config.agents)
        .remove(&key)
        .ok_or_else(|| ApiError::bad_request("agent not found"))?;
    definition.builtin = false;
    let key = format!("custom-{}", uuid::Uuid::new_v4());
    let patch = serde_json::json!({"agents": {&key: &definition}});
    if explicit {
        crate::config::update_project_for_tenant(&tenant, &directory, patch)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
    } else {
        crate::config::update_global_for_tenant(&tenant, patch)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
    }
    definition.api_key = None;
    definition.options.clear();
    Ok(Json(agent::Info { key, definition }))
}

#[derive(Debug, Serialize)]
struct SkillInfo {
    name: String,
    description: String,
    location: String,
    content: String,
}

async fn skill_list(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<SkillInfo>>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let items = skill::all().await;
            Ok(items
                .into_iter()
                .map(|s| SkillInfo {
                    name: s.name,
                    description: s.description,
                    location: s.location,
                    content: s.content,
                })
                .collect::<Vec<_>>())
        })
    })
    .await?;
    Ok(Json(result))
}

#[cfg(test)]
#[path = "misc_tests.rs"]
mod misc_tests;
