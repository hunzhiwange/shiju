//! 识矩私有技能网关传输契约。
//!
//! 统一描述市场、系统和维护中技能，并兼容历史 README 及可空集合的响应形状。

use serde::{Deserialize, Deserializer, Serialize};

use super::{GatewayClient, ShijuCredential};

const SHIJU_SKILLS_PATH: &str = "/v1/shiju/skills";

/// Selects one immutable SHIJU skill version for a remote installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillInstallRequest {
    pub version_id: u64,
}

/// Query for the cursor-based SHIJU private marketplace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillSearchQuery {
    pub query: Option<String>,
    pub cursor: Option<String>,
    pub page_size: u32,
}

/// Query for system skills visible to the active SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSystemSkillQuery {
    pub page: u32,
    pub page_size: u32,
    pub name: Option<String>,
    pub tag_id: Option<u64>,
}

/// Query for skills maintained by the active SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuMaintainedSkillQuery {
    pub page: u32,
    pub page_size: u32,
    pub keyword: Option<String>,
    pub tag_ids: Vec<u64>,
    #[serde(default)]
    pub company_tag_ids: Vec<u64>,
    pub statuses: Vec<String>,
}

/// One SHIJU skill tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillTagDto {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub sort_order: i64,
}

/// Complete SHIJU tag catalog returned by platform and tenant tag endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillTagCatalogDto {
    #[serde(default)]
    pub items: Vec<ShijuSkillTagDto>,
}

/// SHIJU private-market skill summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuMarketplaceSkillDto {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub level: String,
    #[serde(default, deserialize_with = "deserialize_string_or_default")]
    pub modality: String,
    #[serde(default, deserialize_with = "deserialize_string_or_default")]
    pub format: String,
    #[serde(
        default,
        alias = "publisher_company_id",
        deserialize_with = "deserialize_string_or_default"
    )]
    pub company_id: String,
    #[serde(default)]
    pub created_uid: String,
    #[serde(default)]
    pub maintainer_uid: String,
    #[serde(
        default,
        alias = "marketplace_status",
        deserialize_with = "deserialize_string_or_default"
    )]
    pub status: String,
    #[serde(default)]
    pub is_all_company_accessible: u8,
    #[serde(default)]
    pub current_version_id: u64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub is_installed: bool,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub tags: Vec<ShijuSkillTagDto>,
}

/// Cursor page returned by the SHIJU private marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuSkillCursorPageDto {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub items: Vec<ShijuMarketplaceSkillDto>,
    #[serde(default)]
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub has_more: bool,
}

fn deserialize_string_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Option::unwrap_or_default)
}

fn deserialize_vec_or_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Skill installed in the active SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuInstalledSkillDto {
    pub skill_id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub installed_version: String,
    pub current_version: String,
    pub format: String,
    pub upgrade_available: bool,
}

/// Tenant visibility scope returned with system and company skill rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillVisibilityScopeDto {
    pub department_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
}

/// Effective tenant visibility returned by SHIJU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillVisibilityDto {
    pub effective_mode: String,
    pub scope_row: ShijuSkillVisibilityScopeDto,
}

/// SHIJU system skill summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSystemSkillDto {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub is_all_company_accessible: u8,
    pub tags: Vec<ShijuSkillTagDto>,
    pub tenant_visibility: Option<ShijuSkillVisibilityDto>,
}

/// SHIJU company or maintained skill summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuTenantSkillDto {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    #[serde(default)]
    pub is_all_company_accessible: u8,
    pub has_pending_review: bool,
    pub pending_review_version_id: Option<u64>,
    pub tags: Vec<ShijuSkillTagDto>,
    pub tenant_visibility: Option<ShijuSkillVisibilityDto>,
}

/// Numbered page returned by SHIJU tenant catalogs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuSkillPageDto<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

/// Download metadata for the current SHIJU skill version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillVersionContentDto {
    pub checksum: String,
    pub content_type: String,
    pub download_expires_at: i64,
    pub download_url: Option<String>,
    pub version: String,
}

/// Full SHIJU skill detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuSkillDetailDto {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub level: String,
    pub modality: String,
    pub format: String,
    pub company_id: String,
    pub created_uid: String,
    pub maintainer_uid: String,
    pub status: String,
    #[serde(default)]
    pub is_all_company_accessible: u8,
    pub current_version_id: u64,
    pub current_version: Option<ShijuSkillVersionContentDto>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<ShijuSkillTagDto>,
}

/// README content for one SHIJU private skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShijuSkillReadmeDto {
    pub markdown: Option<String>,
}

impl<'de> Deserialize<'de> for ShijuSkillReadmeDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireDto {
            #[serde(default)]
            markdown: Option<String>,
            #[serde(default)]
            content: Option<String>,
        }

        let wire = WireDto::deserialize(deserializer)?;
        let markdown = wire.markdown.or_else(|| wire.content.and_then(legacy_readme_markdown));
        Ok(Self { markdown })
    }
}

fn legacy_readme_markdown(content: String) -> Option<String> {
    #[derive(Deserialize)]
    struct LegacyEnvelope {
        code: i64,
        data: Option<LegacyData>,
    }

    #[derive(Deserialize)]
    struct LegacyData {
        markdown: Option<String>,
    }

    match serde_json::from_str::<LegacyEnvelope>(&content) {
        Ok(envelope) if envelope.code == 200 => envelope.data.and_then(|data| data.markdown),
        Ok(_) => None,
        Err(_) => Some(content),
    }
}

/// SHIJU scan result attached to a published skill version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillScanResultDto {
    pub status: String,
    pub slug_match: bool,
    pub expected_slug: String,
    pub frontmatter_name: String,
    pub frontmatter_name_valid: bool,
}

/// One SHIJU skill version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillVersionDto {
    pub id: u64,
    pub skill_id: u64,
    pub version: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub change_type: String,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub download_expires_at: i64,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub scan_result: Option<ShijuSkillScanResultDto>,
}

#[derive(Serialize)]
struct EmptyRequest {}

impl GatewayClient {
    /// Lists all platform tags available to the active SHIJU account.
    pub async fn shiju_skill_tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/tags"), &[], credential).await
    }

    /// Lists all tenant tags available to the active SHIJU account.
    pub async fn shiju_tenant_skill_tags(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuSkillTagCatalogDto, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/tenant/tags"), &[], credential).await
    }

    /// Searches the cursor-based SHIJU private marketplace.
    pub async fn shiju_skill_search(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSkillSearchQuery,
    ) -> Result<ShijuSkillCursorPageDto, String> {
        let mut query_pairs = vec![("page_size".to_string(), query.page_size.to_string())];
        push_text_filter(&mut query_pairs, "q", query.query.as_deref());
        push_text_filter(&mut query_pairs, "cursor", query.cursor.as_deref());
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/search"), &query_pairs, credential).await
    }

    /// Lists skills installed in the active SHIJU account.
    pub async fn shiju_skill_installed(
        &self,
        credential: &ShijuCredential,
    ) -> Result<Vec<ShijuInstalledSkillDto>, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/installed"), &[], credential).await
    }

    /// Lists SHIJU system skills visible to the active account.
    pub async fn shiju_system_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuSystemSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
        let mut query_pairs = page_query(query.page, query.page_size);
        push_text_filter(&mut query_pairs, "name", query.name.as_deref());
        push_tag_filter(&mut query_pairs, query.tag_id);
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/system"), &query_pairs, credential).await
    }

    /// Lists SHIJU skills maintained by the active account.
    pub async fn shiju_maintained_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
        let mut query_pairs = page_query(query.page, query.page_size);
        push_text_filter(&mut query_pairs, "keyword", query.keyword.as_deref());
        push_tag_ids_filter(&mut query_pairs, "tag_ids", &query.tag_ids);
        push_tag_ids_filter(&mut query_pairs, "company_tag_ids", &query.company_tag_ids);
        push_statuses_filter(&mut query_pairs, &query.statuses);
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/maintained"), &query_pairs, credential)
            .await
    }

    /// Reads one SHIJU private-skill detail.
    pub async fn shiju_skill_detail(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillDetailDto, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/{skill_id}"), &[], credential).await
    }

    /// Reads the README for one SHIJU private skill.
    pub async fn shiju_skill_readme(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<ShijuSkillReadmeDto, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/{skill_id}/readme"), &[], credential)
            .await
    }

    /// Lists published versions for one SHIJU private skill.
    pub async fn shiju_skill_versions(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<Vec<ShijuSkillVersionDto>, String> {
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/{skill_id}/versions"), &[], credential)
            .await
    }

    /// Creates only the remote SHIJU account installation relationship.
    ///
    /// This method does not download a package or write to a local skill directory.
    pub async fn shiju_skill_remote_install(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), String> {
        self.shiju_empty_sensitive_request::<EmptyRequest>(
            reqwest::Method::POST,
            &format!("{SHIJU_SKILLS_PATH}/{skill_id}/install"),
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Creates the remote SHIJU account installation relationship at one version.
    ///
    /// This method does not download a package or write to a local skill directory.
    pub async fn shiju_skill_remote_install_version(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
        version_id: u64,
    ) -> Result<(), String> {
        self.shiju_empty_sensitive_request(
            reqwest::Method::POST,
            &format!("{SHIJU_SKILLS_PATH}/{skill_id}/install"),
            &[],
            Some(&ShijuSkillInstallRequest { version_id }),
            Some(credential),
        )
        .await
    }

    /// Deletes only the remote SHIJU account installation relationship.
    ///
    /// This method does not remove files from a local skill directory.
    pub async fn shiju_skill_remote_uninstall(
        &self,
        credential: &ShijuCredential,
        skill_id: u64,
    ) -> Result<(), String> {
        self.shiju_empty_sensitive_request::<EmptyRequest>(
            reqwest::Method::DELETE,
            &format!("{SHIJU_SKILLS_PATH}/{skill_id}/install"),
            &[],
            None,
            Some(credential),
        )
        .await
    }

    async fn shiju_skill_get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        credential: &ShijuCredential,
    ) -> Result<T, String> {
        self.shiju_account_sensitive_request::<EmptyRequest, T>(
            reqwest::Method::GET,
            path,
            query,
            None,
            Some(credential),
        )
        .await
    }
}

fn page_query(page: u32, page_size: u32) -> Vec<(String, String)> {
    vec![("page".to_string(), page.to_string()), ("page_size".to_string(), page_size.to_string())]
}

fn push_text_filter(query: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        query.push((key.to_string(), value.to_string()));
    }
}

fn push_tag_filter(query: &mut Vec<(String, String)>, tag_id: Option<u64>) {
    if let Some(tag_id) = tag_id {
        query.push(("tag_id".to_string(), tag_id.to_string()));
    }
}

fn push_tag_ids_filter(query: &mut Vec<(String, String)>, key: &str, tag_ids: &[u64]) {
    if !tag_ids.is_empty() {
        let tag_ids = tag_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        query.push((key.to_string(), tag_ids));
    }
}

fn push_statuses_filter(query: &mut Vec<(String, String)>, statuses: &[String]) {
    if !statuses.is_empty() {
        query.push(("statuses".to_string(), statuses.join(",")));
    }
}
