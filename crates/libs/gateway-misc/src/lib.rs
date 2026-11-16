//! 杂项路由模块
#![cfg(not(target_arch = "wasm32"))]

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use gateway_types::ApiError;
use host::{MiscHost, InstanceQuery};
pub mod host;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub key: String,
    #[serde(flatten)]
    pub definition: config_agent::DelegateAgentConfig,
}

pub fn router<S, H: MiscHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/doc", get(not_implemented))
        .route("/command", get(command_list::<H>))
        .route("/agent", get(agent_list::<H>))
        .route("/agent/{key}/copy", post(agent_copy::<H>))
        .route("/skill", get(skill_list::<H>))
}

pub async fn not_implemented() -> Result<Json<Value>, ApiError> {
    Err(ApiError { status: StatusCode::NOT_IMPLEMENTED, message: "not implemented".to_string() })
}

pub async fn command_list<H: MiscHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent_commands::Info>>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result =
        H::with_instance(dir, move || Box::pin(async move { Ok(H::commands().await) }))
            .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct AgentListQuery {
    pub directory: Option<String>,
    #[serde(default)]
    pub include_disabled: bool,
}

pub async fn agent_list<H: MiscHost>(
    Extension(tenant): Extension<vw_api_transport::TenantScope>,
    Query(query): Query<AgentListQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentInfo>>, ApiError> {
    let dir = H::resolve_directory(&InstanceQuery { directory: query.directory }, &headers);
    let result = H::with_tenant_scope(
        tenant,
        H::with_instance(dir, move || {
            Box::pin(async move {
                let mut agents = if query.include_disabled {
                    let config = H::get_current()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?;
                    agent_config::catalog::merged_agent_catalog(&config.agents)
                        .into_iter()
                        .map(|(key, definition)| AgentInfo { key, definition })
                        .collect::<Vec<_>>()
                } else {
                    H::agents().await
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
        }),
    )
    .await?;
    Ok(Json(result))
}

pub async fn agent_copy<H: MiscHost>(
    Extension(tenant): Extension<vw_api_transport::TenantScope>,
    Path(key): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<AgentInfo>, ApiError> {
    // Directory remains accepted for API compatibility, but never selects storage.
    let _ = (query, headers);
    let config = H::get_global(tenant.clone())
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let mut definition = agent_config::catalog::merged_agent_catalog(&config.agents)
        .remove(&key)
        .ok_or_else(|| ApiError::bad_request("agent not found"))?;
    definition.builtin = false;
    let key = format!("custom-{}", uuid::Uuid::new_v4());
    let patch = serde_json::json!({"agents": {&key: &definition}});
    H::update_global(tenant, patch)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    definition.api_key = None;
    definition.options.clear();
    Ok(Json(AgentInfo { key, definition }))
}

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub location: String,
    pub content: String,
}

pub async fn skill_list<H: MiscHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<SkillInfo>>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            Ok(H::skills().await)
        })
    })
    .await?;
    Ok(Json(result))
}

#[cfg(feature = "test-support")]
pub mod test_contract;
