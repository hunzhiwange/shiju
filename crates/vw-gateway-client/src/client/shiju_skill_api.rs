//! SHIJU private-skill gateway transport contract.

use serde::{Deserialize, Serialize};

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

/// Query for company skills visible to the active SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuCompanySkillQuery {
    pub page: u32,
    pub page_size: u32,
    pub keyword: Option<String>,
    pub tag_id: Option<u64>,
    pub status: Option<String>,
}

/// Query for skills maintained by the active SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuMaintainedSkillQuery {
    pub page: u32,
    pub page_size: u32,
    pub keyword: Option<String>,
    pub tag_id: Option<u64>,
}

/// One SHIJU skill tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuSkillTagDto {
    pub id: u64,
    pub name: String,
    pub sort_order: i64,
}

/// SHIJU private-market skill summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuMarketplaceSkillDto {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub level: String,
    pub modality: String,
    pub format: String,
    pub company_id: String,
    pub created_uid: String,
    pub maintainer_uid: String,
    pub status: String,
    pub is_all_company_accessible: u8,
    pub current_version_id: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<i64>,
    pub is_installed: bool,
    pub tags: Vec<ShijuSkillTagDto>,
}

/// Cursor page returned by the SHIJU private marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShijuSkillCursorPageDto {
    pub items: Vec<ShijuMarketplaceSkillDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
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
    pub level: String,
    pub modality: String,
    pub format: String,
    pub company_id: String,
    pub created_uid: String,
    pub maintainer_uid: String,
    pub status: String,
    pub is_all_company_accessible: u8,
    pub current_version_id: u64,
    pub current_version: Option<ShijuSkillVersionContentDto>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<ShijuSkillTagDto>,
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

    /// Lists SHIJU company skills visible to the active account.
    pub async fn shiju_company_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuCompanySkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
        let mut query_pairs = page_query(query.page, query.page_size);
        push_text_filter(&mut query_pairs, "keyword", query.keyword.as_deref());
        push_tag_filter(&mut query_pairs, query.tag_id);
        push_text_filter(&mut query_pairs, "status", query.status.as_deref());
        self.shiju_skill_get(&format!("{SHIJU_SKILLS_PATH}/company"), &query_pairs, credential)
            .await
    }

    /// Lists SHIJU skills maintained by the active account.
    pub async fn shiju_maintained_skills(
        &self,
        credential: &ShijuCredential,
        query: &ShijuMaintainedSkillQuery,
    ) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
        let mut query_pairs = page_query(query.page, query.page_size);
        push_text_filter(&mut query_pairs, "keyword", query.keyword.as_deref());
        push_tag_filter(&mut query_pairs, query.tag_id);
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
