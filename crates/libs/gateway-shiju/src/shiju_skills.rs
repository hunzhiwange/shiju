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
use {gateway_shiju_account::SHIJU_CREDENTIAL_HEADER, gateway_shiju_account::ShijuCredential, gateway_shiju_skill::ShijuInstalledSkillDto, gateway_shiju_skill::ShijuMaintainedSkillQuery, gateway_shiju_skill::ShijuSkillCursorPageDto, gateway_shiju_skill::ShijuSkillDetailDto, gateway_shiju_skill::ShijuSkillPageDto, gateway_shiju_skill::ShijuSkillReadmeDto, gateway_shiju_skill::ShijuSkillSearchQuery, gateway_shiju_skill::ShijuSkillTagCatalogDto, gateway_shiju_skill::ShijuSkillVersionDto, gateway_shiju_skill::ShijuSystemSkillDto, gateway_shiju_skill::ShijuSystemSkillQuery, gateway_shiju_skill::ShijuTenantSkillDto};

use gateway_types::ApiError;


/// 上游技能服务端口；宿主注入真实客户端并保留原错误映射。
pub trait ShijuSkillService: Send + Sync + 'static {
    fn search(&self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery) -> impl std::future::Future<Output = Result<ShijuSkillCursorPageDto, ApiError>> + Send;
    fn installed(&self,
        credential: &ShijuCredential) -> impl std::future::Future<Output = Result<Vec<ShijuInstalledSkillDto>, ApiError>> + Send;
    fn tags(&self,
        credential: &ShijuCredential) -> impl std::future::Future<Output = Result<ShijuSkillTagCatalogDto, ApiError>> + Send;
    fn tenant_tags(&self,
        credential: &ShijuCredential) -> impl std::future::Future<Output = Result<ShijuSkillTagCatalogDto, ApiError>> + Send;
    fn system(&self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery) -> impl std::future::Future<Output = Result<ShijuSkillPageDto<ShijuSystemSkillDto>, ApiError>> + Send;
    fn maintained(&self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery) -> impl std::future::Future<Output = Result<ShijuSkillPageDto<ShijuTenantSkillDto>, ApiError>> + Send;
    fn detail(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> impl std::future::Future<Output = Result<ShijuSkillDetailDto, ApiError>> + Send;
    fn readme(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> impl std::future::Future<Output = Result<ShijuSkillReadmeDto, ApiError>> + Send;
    fn versions(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> impl std::future::Future<Output = Result<Vec<ShijuSkillVersionDto>, ApiError>> + Send;
    fn remote_install(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
    fn remote_install_version(&self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
    fn remote_uninstall(&self,
        credential: &ShijuCredential,
        skill_id: u64) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
}

#[derive(Deserialize)]
struct InstallRequest { version_id: u64 }

pub fn router_with_client<S, C>(client: C) -> Router<S>
where S: Clone + Send + Sync + 'static, C: ShijuSkillService,
{
    Router::new()
        .route("/shiju/skills/search", get(search_get::<C>))
        .route("/shiju/skills/installed", get(installed_get::<C>))
        .route("/shiju/skills/tags", get(tags_get::<C>))
        .route("/shiju/skills/tenant/tags", get(tenant_tags_get::<C>))
        .route("/shiju/skills/system", get(system_get::<C>))
        .route("/shiju/skills/maintained", get(maintained_get::<C>))
        .route("/shiju/skills/{skill_id}", get(detail_get::<C>))
        .route("/shiju/skills/{skill_id}/readme", get(readme_get::<C>))
        .route("/shiju/skills/{skill_id}/versions", get(versions_get::<C>))
        .route("/shiju/skills/{skill_id}/install", post(install_post::<C>).delete(install_delete::<C>))
        .layer(Extension(Arc::new(client)))
}

#[derive(Deserialize)]
struct SearchParams {
    #[serde(rename = "q")]
    query: Option<String>,
    cursor: Option<String>,
    page_size: u32,
    tag_ids: Option<u64>,
    company_tag_ids: Option<u64>,
}

#[derive(Deserialize)]
struct SystemParams {
    page: u32,
    page_size: u32,
    name: Option<String>,
    tag_id: Option<u64>,
}

#[derive(Deserialize)]
struct MaintainedParams {
    page: u32,
    page_size: u32,
    keyword: Option<String>,
    tag_ids: Option<String>,
    company_tag_ids: Option<String>,
    statuses: Option<String>,
}

async fn search_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<ShijuSkillCursorPageDto>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuSkillSearchQuery {
        query: params.query,
        cursor: params.cursor,
        page_size: params.page_size,
        tag_ids: params.tag_ids,
        company_tag_ids: params.company_tag_ids,
    };
    client.search(&credential, &query).await.map(Json)
}

async fn installed_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ShijuInstalledSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    client.installed(&credential).await.map(Json)
}

async fn tags_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
) -> Result<Json<ShijuSkillTagCatalogDto>, ApiError> {
    let credential = credential(&headers)?;
    client.tags(&credential).await.map(Json)
}

async fn tenant_tags_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
) -> Result<Json<ShijuSkillTagCatalogDto>, ApiError> {
    let credential = credential(&headers)?;
    client.tenant_tags(&credential).await.map(Json)
}

async fn system_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
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
    client.system(&credential, &query).await.map(Json)
}

async fn maintained_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Query(params): Query<MaintainedParams>,
) -> Result<Json<ShijuSkillPageDto<ShijuTenantSkillDto>>, ApiError> {
    let credential = credential(&headers)?;
    let query = ShijuMaintainedSkillQuery {
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword,
        tag_ids: parse_tag_ids(params.tag_ids.as_deref(), "tag_ids")?,
        company_tag_ids: parse_tag_ids(params.company_tag_ids.as_deref(), "company_tag_ids")?,
        statuses: parse_statuses(params.statuses.as_deref())?,
    };
    client.maintained(&credential, &query).await.map(Json)
}

async fn detail_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<ShijuSkillDetailDto>, ApiError> {
    let credential = credential(&headers)?;
    client.detail(&credential, skill_id).await.map(Json)
}

async fn readme_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<ShijuSkillReadmeDto>, ApiError> {
    let credential = credential(&headers)?;
    client.readme(&credential, skill_id).await.map(Json)
}

async fn versions_get<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<Json<Vec<ShijuSkillVersionDto>>, ApiError> {
    let credential = credential(&headers)?;
    client.versions(&credential, skill_id).await.map(Json)
}

async fn install_post<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
    body: Option<Json<InstallRequest>>,
) -> Result<(), ApiError> {
    let credential = credential(&headers)?;
    match body {
        Some(Json(body)) => {
            client.remote_install_version(&credential, skill_id, body.version_id).await
        }
        None => client.remote_install(&credential, skill_id).await,
    }

}

async fn install_delete<C: ShijuSkillService>(
    Extension(client): Extension<Arc<C>>,
    headers: HeaderMap,
    Path(skill_id): Path<u64>,
) -> Result<(), ApiError> {
    let credential = credential(&headers)?;
    client.remote_uninstall(&credential, skill_id).await
}

fn credential(headers: &HeaderMap) -> Result<ShijuCredential, ApiError> {
    let raw = headers
        .get(SHIJU_CREDENTIAL_HEADER)
        .ok_or_else(|| ApiError::bad_request("SHIJU credential header is required"))?
        .to_str()
        .map_err(|_| ApiError::bad_request("SHIJU credential header is invalid"))?;
    ShijuCredential::new(raw).map_err(|_| ApiError::bad_request("SHIJU credential cannot be empty"))
}

fn parse_tag_ids(value: Option<&str>, name: &str) -> Result<Vec<u64>, ApiError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(|tag_id| {
            let tag_id = tag_id.trim();
            if tag_id.is_empty() {
                return Err(ApiError::bad_request(format!(
                    "SHIJU skill {name} cannot contain empty items"
                )));
            }
            let tag_id = tag_id.parse::<u64>().map_err(|_| {
                ApiError::bad_request(format!(
                    "SHIJU skill {name} must contain only positive integers"
                ))
            })?;
            if tag_id == 0 {
                return Err(ApiError::bad_request(format!(
                    "SHIJU skill {name} must contain only positive integers"
                )));
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
