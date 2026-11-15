use serde::Serialize;
use gateway_shiju_account::ShijuCredential;
use super::{GatewayShijuSkillClient, types::*};

const SHIJU_SKILLS_PATH: &str = "/v1/shiju/skills";

#[derive(Serialize)]
struct EmptyRequest {}

impl GatewayShijuSkillClient {
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
        push_optional_u64_filter(&mut query_pairs, "tag_ids", query.tag_ids);
        push_optional_u64_filter(&mut query_pairs, "company_tag_ids", query.company_tag_ids);
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone()).shiju_empty_sensitive_request::<EmptyRequest>(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone()).shiju_empty_sensitive_request(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone()).shiju_empty_sensitive_request::<EmptyRequest>(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone()).shiju_account_sensitive_request::<EmptyRequest, T>(
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

fn push_optional_u64_filter(query: &mut Vec<(String, String)>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
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
