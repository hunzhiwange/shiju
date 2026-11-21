//! SHIJU 私有技能服务的上游客户端。

use std::fmt;
use std::time::Duration;

use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use {gateway_shiju_account::ShijuCredential, gateway_shiju_skill::ShijuInstalledSkillDto, gateway_shiju_skill::ShijuMaintainedSkillQuery, gateway_shiju_skill::ShijuSkillCursorPageDto, gateway_shiju_skill::ShijuSkillDetailDto, gateway_shiju_skill::ShijuSkillPageDto, gateway_shiju_skill::ShijuSkillReadmeDto, gateway_shiju_skill::ShijuSkillSearchQuery, gateway_shiju_skill::ShijuSkillTagCatalogDto, gateway_shiju_skill::ShijuSkillVersionDto, gateway_shiju_skill::ShijuSystemSkillDto, gateway_shiju_skill::ShijuSystemSkillQuery, gateway_shiju_skill::ShijuTenantSkillDto};

const SHIJU_SKILL_BASE_URL: &str = "https://gateway.shiju.cloud/sj/skill-hub/api/v1/";
const MAX_PAGE_SIZE: u32 = 100;
const MAINTAINED_SKILL_STATUSES: [&str; 4] =
    ["draft", "pending_review", "published", "unpublished"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
/// 远程安装指定技能版本的请求体。
pub struct ShijuSkillInstallRequest {
    pub version_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShijuSkillErrorKind {
    InvalidInput,
    AccessDenied,
    Upstream,
}

#[derive(Clone, PartialEq, Eq)]
/// SHIJU 技能接口的输入校验或上游错误。
pub struct ShijuSkillError {
    kind: ShijuSkillErrorKind,
    message: String,
}

impl ShijuSkillError {
    fn invalid(message: impl Into<String>) -> Self {
        Self { kind: ShijuSkillErrorKind::InvalidInput, message: message.into() }
    }

    fn upstream(message: impl Into<String>) -> Self {
        Self { kind: ShijuSkillErrorKind::Upstream, message: message.into() }
    }

    /// 判断错误是否来自本地输入校验。
    pub fn is_invalid_input(&self) -> bool {
        self.kind == ShijuSkillErrorKind::InvalidInput
    }
}

impl fmt::Debug for ShijuSkillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ShijuSkillError").field(&self.message).finish()
    }
}

impl fmt::Display for ShijuSkillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ShijuSkillError {}

#[derive(Clone)]
/// SHIJU 私有技能服务客户端。
pub struct ShijuSkillClient {
    http: reqwest::Client,
    base_url: Url,
}

impl ShijuSkillClient {
    /// 使用固定生产端点创建客户端。
    pub fn production() -> Result<Self, ShijuSkillError> {
        Self::new(SHIJU_SKILL_BASE_URL)
    }

    /// 使用显式基础地址创建客户端。
    pub fn with_base_url(base_url: &str) -> Result<Self, ShijuSkillError> {
        Self::new(base_url)
    }

    fn new(base_url: &str) -> Result<Self, ShijuSkillError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| ShijuSkillError::upstream("failed to create SHIJU skill client"))?;
        let mut base_url = Url::parse(base_url)
            .map_err(|_| ShijuSkillError::invalid("SHIJU skill upstream URL is invalid"))?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Ok(Self { http, base_url })
    }

    /// 按关键词、标签和分页条件搜索可见技能。
    pub async fn search(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery,
    ) -> Result<ShijuSkillCursorPageDto, ShijuSkillError> {
        validate_page_size(query.page_size)?;
        validate_optional_text(query.query.as_deref(), "q")?;
        validate_optional_text(query.cursor.as_deref(), "cursor")?;
        validate_single_tag_id(query.tag_ids, "tag_ids")?;
        validate_single_tag_id(query.company_tag_ids, "company_tag_ids")?;
        let mut parameters = vec![("page_size", query.page_size.to_string())];
        push_text_filter(&mut parameters, "q", query.query.as_deref());
        push_text_filter(&mut parameters, "cursor", query.cursor.as_deref());
        push_optional_u64_filter(&mut parameters, "tag_ids", query.tag_ids);
        push_optional_u64_filter(&mut parameters, "company_tag_ids", query.company_tag_ids);
        push_text_filter(&mut parameters, "distribution_source", query.distribution_source.map(|source| source.as_str()));
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SearchResponse {
            Page(ShijuSkillCursorPageDto),
            Items(Vec<gateway_shiju_skill::ShijuMarketplaceSkillDto>),
        }
        match self.get("marketplace/skills", &parameters, credential).await? {
            SearchResponse::Page(page) => Ok(page),
            SearchResponse::Items(items) => Ok(ShijuSkillCursorPageDto {
                items,
                next_cursor: None,
                has_more: false,
            }),
        }
    }

    /// 列出当前租户已安装技能。
    pub async fn installed(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuInstalledSkillDto>, ShijuSkillError> {
        self.get("me/skills", &[("include_card_metadata", "true".into())], credential).await
    }

    /// 列出完整的平台标签目录。
    pub async fn tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, ShijuSkillError> {
        self.get("tags", &[], credential).await
    }

    /// 列出完整的公司标签目录。
    pub async fn tenant_tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, ShijuSkillError> {
        self.get("tenant/tags", &[], credential).await
    }

    /// 列出可见的系统技能。
    pub async fn system(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, ShijuSkillError> {
        validate_page(query.page, query.page_size)?;
        validate_optional_text(query.name.as_deref(), "name")?;
        validate_tag_id(query.tag_id)?;
        let mut parameters = page_parameters(query.page, query.page_size);
        push_text_filter(&mut parameters, "name", query.name.as_deref());
        push_tag_filter(&mut parameters, query.tag_id);
        self.get("tenant/system-skills", &parameters, credential).await
    }

    /// 列出当前用户维护的技能及指定状态集合。
    pub async fn maintained(
        &self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, ShijuSkillError> {
        validate_page(query.page, query.page_size)?;
        validate_optional_text(query.keyword.as_deref(), "keyword")?;
        validate_tag_ids(&query.tag_ids, "tag_ids")?;
        validate_tag_ids(&query.company_tag_ids, "company_tag_ids")?;
        validate_statuses(&query.statuses)?;
        let mut parameters = page_parameters(query.page, query.page_size);
        push_text_filter(&mut parameters, "keyword", query.keyword.as_deref());
        push_tag_ids_filter(&mut parameters, "tag_ids", &query.tag_ids);
        push_tag_ids_filter(&mut parameters, "company_tag_ids", &query.company_tag_ids);
        push_statuses_filter(&mut parameters, &query.statuses);
        self.get("me/maintained-skills", &parameters, credential).await
    }

    /// 读取单个技能详情。
    pub async fn detail(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillDetailDto, ShijuSkillError> {
        validate_skill_id(skill_id)?;
        let legacy_path = format!("skills/{skill_id}");
        let market_path = format!("marketplace/skills/{skill_id}");
        let (legacy, market) = futures_util::future::join(
            self.get::<ShijuSkillDetailDto>(&legacy_path, &[], credential),
            self.get::<ShijuSkillDetailDto>(&market_path, &[], credential),
        )
        .await;
        let market = market.ok().filter(|detail| detail.id == skill_id);
        match legacy {
            Ok(mut detail) => {
                if let Some(catalog) = market.filter(|catalog| catalog.id == detail.id) {
                    let metadata = &mut detail.marketplace;
                    let catalog_metadata = catalog.marketplace;
                    metadata.publisher_company_id = metadata.publisher_company_id.take().or(catalog_metadata.publisher_company_id);
                    metadata.publisher_company_name = metadata.publisher_company_name.take().or(catalog_metadata.publisher_company_name);
                    metadata.is_platform_publisher = metadata.is_platform_publisher.take().or(catalog_metadata.is_platform_publisher);
                    metadata.publisher_kind = metadata.publisher_kind.take().or(catalog_metadata.publisher_kind);
                    metadata.developer_display_name = metadata.developer_display_name.take().or(catalog_metadata.developer_display_name);
                    metadata.creator_display_name = metadata.creator_display_name.take().or(catalog_metadata.creator_display_name);
                    metadata.marketplace_submission_id = metadata.marketplace_submission_id.take().or(catalog_metadata.marketplace_submission_id);
                    metadata.marketplace_status = metadata.marketplace_status.take().or(catalog_metadata.marketplace_status);
                    metadata.marketplace_version = metadata.marketplace_version.take().or(catalog_metadata.marketplace_version);
                    metadata.distribution_source = metadata.distribution_source.take().or(catalog_metadata.distribution_source);
                    metadata.content_type = metadata.content_type.take().or(catalog_metadata.content_type);
                    metadata.is_installed = metadata.is_installed.take().or(catalog_metadata.is_installed);
                    metadata.icon_url = metadata.icon_url.take().or(catalog_metadata.icon_url);
                    if detail.tags.is_empty() {
                        detail.tags = catalog.tags;
                    }
                }
                Ok(detail)
            }
            Err(error) if error.kind == ShijuSkillErrorKind::Upstream => market.ok_or(error),
            Err(error) => Err(error),
        }
    }

    /// 读取单个技能说明文档。
    pub async fn readme(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillReadmeDto, ShijuSkillError> {
        validate_skill_id(skill_id)?;
        let result = self
            .get::<ShijuSkillReadmeDto>(&format!("skills/{skill_id}/readme"), &[], credential)
            .await;
        match &result {
            Ok(readme) if readme.markdown.as_ref().is_some_and(|text| !text.trim().is_empty()) => {
                return result;
            }
            Err(error) if error.kind != ShijuSkillErrorKind::Upstream => return result,
            _ => {}
        }
        self.get(&format!("marketplace/skills/{skill_id}/readme"), &[], credential)
            .await
            .or(result)
    }

    /// 列出技能的可安装版本。
    pub async fn versions(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<Vec<ShijuSkillVersionDto>, ShijuSkillError> {
        validate_skill_id(skill_id)?;
        self.get(&format!("skills/{skill_id}/versions"), &[], credential).await
    }

    /// 安装服务端选择的默认技能版本。
    pub async fn remote_install(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), ShijuSkillError> {
        self.remote_install_from(credential, skill_id, None).await
    }

    pub async fn remote_install_from(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
        source: Option<gateway_shiju_skill::ShijuSkillSource>,
    ) -> Result<(), ShijuSkillError> {
        validate_skill_id(skill_id)?;
        let collection = if source == Some(gateway_shiju_skill::ShijuSkillSource::Marketplace) {
            "marketplace/skills"
        } else {
            "skills"
        };
        self.mutate::<ShijuSkillInstallRequest>(
            Method::POST,
            &format!("{collection}/{skill_id}/install"),
            credential,
            None,
        )
        .await
    }

    /// 安装调用方指定的技能版本。
    pub async fn remote_install_version(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64,
    ) -> Result<(), ShijuSkillError> {
        validate_skill_id(skill_id)?;
        validate_version_id(version_id)?;
        self.mutate(
            Method::POST,
            &format!("skills/{skill_id}/install"),
            credential,
            Some(&ShijuSkillInstallRequest { version_id }),
        )
        .await
    }

    /// 卸载当前租户中的指定技能。
    pub async fn remote_uninstall(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), ShijuSkillError> {
        self.remote_uninstall_from(credential, skill_id, None).await
    }

    pub async fn remote_uninstall_from(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
        source: Option<gateway_shiju_skill::ShijuSkillSource>,
    ) -> Result<(), ShijuSkillError> {
        validate_skill_id(skill_id)?;
        let collection = if source == Some(gateway_shiju_skill::ShijuSkillSource::Marketplace) {
            "marketplace/skills"
        } else {
            "skills"
        };
        self.mutate::<ShijuSkillInstallRequest>(
            Method::DELETE,
            &format!("{collection}/{skill_id}/install"),
            credential,
            None,
        )
        .await
    }

    async fn get<T>(
        &self,
        path: &str,
        query: &[(&str, String)],
        credential: &ShijuCredential,
    ) -> Result<T, ShijuSkillError>
    where
        T: DeserializeOwned,
    {
        let response = self
            .http
            .get(self.endpoint(path)?)
            .header("X-Api-Key", credential.expose())
            .query(query)
            .send()
            .await
            .map_err(|_| ShijuSkillError::upstream("SHIJU skill upstream request failed"))?;
        decode_response(response).await
    }

    async fn mutate<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        credential: &ShijuCredential,
        body: Option<&B>,
    ) -> Result<(), ShijuSkillError> {
        let mut request = self
            .http
            .request(method, self.endpoint(path)?)
            .header("X-Api-Key", credential.expose());
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ShijuSkillError::upstream("SHIJU skill upstream request failed"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(http_error(status));
        }
        let body = response
            .bytes()
            .await
            .map_err(|_| ShijuSkillError::upstream("SHIJU skill upstream response failed"))?;
        if body.is_empty() {
            return Ok(());
        }
        let envelope = serde_json::from_slice::<MutationEnvelope>(&body)
            .map_err(|_| ShijuSkillError::upstream("SHIJU skill upstream response is invalid"))?;
        if envelope.code != 200 {
            return Err(business_error(envelope.code));
        }
        Ok(())
    }

    fn endpoint(&self, path: &str) -> Result<Url, ShijuSkillError> {
        self.base_url
            .join(path)
            .map_err(|_| ShijuSkillError::invalid("SHIJU skill upstream path is invalid"))
    }
}

async fn decode_response<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, ShijuSkillError> {
    let status = response.status();
    if !status.is_success() {
        return Err(http_error(status));
    }
    let envelope = response.json::<ApiEnvelope>().await.map_err(|error| {
        tracing::warn!(error = %error, "failed to decode SHIJU skill upstream response");
        ShijuSkillError::upstream("SHIJU skill upstream response is invalid")
    })?;
    if envelope.code != 200 {
        return Err(business_error(envelope.code));
    }
    let data = envelope
        .data
        .ok_or_else(|| ShijuSkillError::upstream("SHIJU skill upstream response is missing data"))?;
    serde_json::from_value(data).map_err(|_| {
        tracing::warn!("failed to decode SHIJU skill upstream data");
        ShijuSkillError::upstream("SHIJU skill upstream response is invalid")
    })
}

fn http_error(status: StatusCode) -> ShijuSkillError {
    upstream_status_error(
        i64::from(status.as_u16()),
        format!("SHIJU skill upstream request failed: HTTP {}", status.as_u16()),
    )
}

fn business_error(code: i64) -> ShijuSkillError {
    upstream_status_error(code, format!("SHIJU skill upstream returned business error {code}"))
}

fn upstream_status_error(code: i64, message: String) -> ShijuSkillError {
    let kind = match code {
        401 | 403 => ShijuSkillErrorKind::AccessDenied,
        _ => ShijuSkillErrorKind::Upstream,
    };
    ShijuSkillError { kind, message }
}

fn validate_page(page: u32, page_size: u32) -> Result<(), ShijuSkillError> {
    if page == 0 {
        return Err(ShijuSkillError::invalid("SHIJU skill page must be greater than zero"));
    }
    validate_page_size(page_size)
}

fn validate_page_size(page_size: u32) -> Result<(), ShijuSkillError> {
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(ShijuSkillError::invalid(format!(
            "SHIJU skill page_size must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    Ok(())
}

fn validate_skill_id(skill_id: u64) -> Result<(), ShijuSkillError> {
    if skill_id == 0 {
        Err(ShijuSkillError::invalid("SHIJU skill id must be greater than zero"))
    } else {
        Ok(())
    }
}

fn validate_version_id(version_id: u64) -> Result<(), ShijuSkillError> {
    if version_id == 0 {
        Err(ShijuSkillError::invalid("SHIJU skill version id must be greater than zero"))
    } else {
        Ok(())
    }
}

fn validate_tag_id(tag_id: Option<u64>) -> Result<(), ShijuSkillError> {
    if tag_id == Some(0) {
        Err(ShijuSkillError::invalid("SHIJU skill tag_id must be greater than zero"))
    } else {
        Ok(())
    }
}

fn validate_single_tag_id(tag_id: Option<u64>, name: &str) -> Result<(), ShijuSkillError> {
    if tag_id == Some(0) {
        Err(ShijuSkillError::invalid(format!("SHIJU skill {name} must be greater than zero")))
    } else {
        Ok(())
    }
}

fn validate_tag_ids(tag_ids: &[u64], name: &str) -> Result<(), ShijuSkillError> {
    if tag_ids.contains(&0) {
        Err(ShijuSkillError::invalid(format!(
            "SHIJU skill {name} must contain only positive integers"
        )))
    } else {
        Ok(())
    }
}

fn validate_statuses(statuses: &[String]) -> Result<(), ShijuSkillError> {
    if statuses.iter().any(|status| !MAINTAINED_SKILL_STATUSES.contains(&status.as_str())) {
        Err(ShijuSkillError::invalid("SHIJU skill statuses contains an unsupported value"))
    } else {
        Ok(())
    }
}

fn validate_optional_text(value: Option<&str>, name: &str) -> Result<(), ShijuSkillError> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        Err(ShijuSkillError::invalid(format!("SHIJU skill {name} filter cannot be blank")))
    } else {
        Ok(())
    }
}

fn page_parameters(page: u32, page_size: u32) -> Vec<(&'static str, String)> {
    vec![("page", page.to_string()), ("page_size", page_size.to_string())]
}

fn push_text_filter<'a>(query: &mut Vec<(&'a str, String)>, key: &'a str, value: Option<&str>) {
    if let Some(value) = value {
        query.push((key, value.to_string()));
    }
}

fn push_optional_u64_filter<'a>(
    query: &mut Vec<(&'a str, String)>,
    key: &'a str,
    value: Option<u64>,
) {
    if let Some(value) = value {
        query.push((key, value.to_string()));
    }
}

fn push_tag_filter(query: &mut Vec<(&str, String)>, tag_id: Option<u64>) {
    if let Some(tag_id) = tag_id {
        query.push(("tag_id", tag_id.to_string()));
    }
}

fn push_tag_ids_filter<'a>(query: &mut Vec<(&'a str, String)>, key: &'a str, tag_ids: &[u64]) {
    if !tag_ids.is_empty() {
        let tag_ids = tag_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        query.push((key, tag_ids));
    }
}

fn push_statuses_filter(query: &mut Vec<(&str, String)>, statuses: &[String]) {
    if !statuses.is_empty() {
        query.push(("statuses", statuses.join(",")));
    }
}

#[derive(Deserialize)]
struct ApiEnvelope {
    code: i64,
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct MutationEnvelope {
    code: i64,
}

#[cfg(test)]
#[path = "shiju_tests.rs"]
mod shiju_tests;
