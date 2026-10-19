//! SHIJU 私有技能网关接口，处理检索、详情、版本与安装操作。
//!
//! 安装与卸载由技能客户端执行，本层只解析分页、凭据和路由参数并映射错误。
//!
//! SHIJU private-skill gateway API.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use vw_gateway_client::{
    SHIJU_CREDENTIAL_HEADER, ShijuCompanySkillQuery, ShijuCredential, ShijuInstalledSkillDto,
    ShijuMaintainedSkillQuery, ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto,
    ShijuSkillReadmeDto, ShijuSkillSearchQuery, ShijuSkillVersionDto, ShijuSystemSkillDto,
    ShijuSystemSkillQuery, ShijuTenantSkillDto,
};

use crate::gateway::ApiError;
use crate::skills::shiju::{ShijuSkillClient, ShijuSkillError, ShijuSkillInstallRequest};

/// 使用生产 SHIJU 技能客户端构建路由。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let client = ShijuSkillClient::production()
        .unwrap_or_else(|error| panic!("failed to initialize SHIJU skill client: {error}"));
    router_with_client(client)
}

/// 使用显式技能客户端构建路由。
pub(crate) fn router_with_client<S>(client: ShijuSkillClient) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/shiju/skills/search", get(search_get))
        .route("/shiju/skills/installed", get(installed_get))
        .route("/shiju/skills/system", get(system_get))
        .route("/shiju/skills/company", get(company_get))
        .route("/shiju/skills/maintained", get(maintained_get))
        .route("/shiju/skills/{skill_id}", get(detail_get))
        .route("/shiju/skills/{skill_id}/readme", get(readme_get))
        .route("/shiju/skills/{skill_id}/versions", get(versions_get))
        .route("/shiju/skills/{skill_id}/install", post(install_post).delete(install_delete))
        .layer(Extension(Arc::new(client)))
}

#[derive(Deserialize)]
struct SearchParams {
    #[serde(rename = "q")]
    query: Option<String>,
    cursor: Option<String>,
    page_size: u32,
}

#[derive(Deserialize)]
struct SystemParams {
    page: u32,
    page_size: u32,
    name: Option<String>,
    tag_id: Option<u64>,
}

#[derive(Deserialize)]
struct CompanyParams {
    page: u32,
    page_size: u32,
    keyword: Option<String>,
    tag_id: Option<u64>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct MaintainedParams {
    page: u32,
    page_size: u32,
    keyword: Option<String>,
    tag_ids: Option<String>,
    statuses: Option<String>,
}

async fn search_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<ShijuSkillCursorPageDto>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuSkillSearchQuery {
        query: params.query,
        cursor: params.cursor,
        page_size: params.page_size,
    };
    client.search(&credential, &query).await.map(Json).map_err(map_error)
}

async fn installed_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ShijuInstalledSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    client.installed(&credential).await.map(Json).map_err(map_error)
}

async fn system_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Query(params): Query<SystemParams>,
) -> Result<Json<ShijuSkillPageDto<ShijuSystemSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuSystemSkillQuery {
        page: params.page,
        page_size: params.page_size,
        name: params.name,
        tag_id: params.tag_id,
    };
    client.system(&credential, &query).await.map(Json).map_err(map_error)
}

async fn company_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Query(params): Query<CompanyParams>,
) -> Result<Json<ShijuSkillPageDto<ShijuTenantSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuCompanySkillQuery {
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword,
        tag_id: params.tag_id,
        status: params.status,
    };
    client.company(&credential, &query).await.map(Json).map_err(map_error)
}

async fn maintained_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Query(params): Query<MaintainedParams>,
) -> Result<Json<ShijuSkillPageDto<ShijuTenantSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuMaintainedSkillQuery {
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword,
        tag_ids: parse_tag_ids(params.tag_ids.as_deref())?,
        statuses: parse_statuses(params.statuses.as_deref())?,
    };
    client.maintained(&credential, &query).await.map(Json).map_err(map_error)
}

async fn detail_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<ShijuSkillDetailDto>, ApiError> {
    let credential = credential(&headers)?;
    client.detail(&credential, skill_id).await.map(Json).map_err(map_error)
}

async fn readme_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<ShijuSkillReadmeDto>, ApiError> {
    let credential = credential(&headers)?;
    client.readme(&credential, skill_id).await.map(Json).map_err(map_error)
}

async fn versions_get(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<Vec<ShijuSkillVersionDto>>, ApiError> {
    let credential = credential(&headers)?;
    client.versions(&credential, skill_id).await.map(Json).map_err(map_error)
}

async fn install_post(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
    body: Option<Json<ShijuSkillInstallRequest>>,
) -> Result<(), ApiError> {
    let credential = credential(&headers)?;
    match body {
        Some(Json(body)) => {
            client.remote_install_version(&credential, skill_id, body.version_id).await
        }
        None => client.remote_install(&credential, skill_id).await,
    }
    .map_err(map_error)
}

async fn install_delete(
    Extension(client): Extension<Arc<ShijuSkillClient>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<(), ApiError> {
    let credential = credential(&headers)?;
    client.remote_uninstall(&credential, skill_id).await.map_err(map_error)
}

fn credential(headers: &HeaderMap) -> Result<ShijuCredential, ApiError> {
    let raw = headers
        .get(SHIJU_CREDENTIAL_HEADER)
        .ok_or_else(|| ApiError::bad_request("SHIJU credential header is required"))?
        .to_str()
        .map_err(|_| ApiError::bad_request("SHIJU credential header is invalid"))?;
    ShijuCredential::new(raw).map_err(|_| ApiError::bad_request("SHIJU credential cannot be empty"))
}

fn map_error(error: ShijuSkillError) -> ApiError {
    if error.is_invalid_input() {
        ApiError::bad_request(error.to_string())
    } else {
        ApiError::internal("SHIJU skill upstream request failed")
    }
}

fn parse_tag_ids(value: Option<&str>) -> Result<Vec<u64>, ApiError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(|tag_id| {
            let tag_id = tag_id.trim();
            if tag_id.is_empty() {
                return Err(ApiError::bad_request(
                    "SHIJU skill tag_ids cannot contain empty items",
                ));
            }
            let tag_id = tag_id.parse::<u64>().map_err(|_| {
                ApiError::bad_request("SHIJU skill tag_ids must contain only positive integers")
            })?;
            if tag_id == 0 {
                return Err(ApiError::bad_request(
                    "SHIJU skill tag_ids must contain only positive integers",
                ));
            }
            Ok(tag_id)
        })
        .collect()
}

fn parse_statuses(value: Option<&str>) -> Result<Vec<String>, ApiError> {
    const ALLOWED_STATUSES: [&str; 4] = ["draft", "pending_review", "published", "unpublished"];

    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(|status| {
            let status = status.trim();
            if status.is_empty() {
                return Err(ApiError::bad_request(
                    "SHIJU skill statuses cannot contain empty items",
                ));
            }
            if !ALLOWED_STATUSES.contains(&status) {
                return Err(ApiError::bad_request(format!(
                    "SHIJU skill statuses contains unsupported value: {status}"
                )));
            }
            Ok(status.to_string())
        })
        .collect()
}

#[cfg(test)]
#[path = "shiju_skills_tests.rs"]
mod shiju_skills_tests;
