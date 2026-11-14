use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::{inventory_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillInventoryError { Transport, Decode, InvalidId, Http { status: u16 } }
impl std::fmt::Display for SkillInventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill inventory transport failed"),
            Self::Decode => f.write_str("skill inventory response invalid"),
            Self::InvalidId => f.write_str("invalid installed skill identity"),
            Self::Http { status } => write!(f, "skill inventory HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillInventoryError {}
impl GatewayClient {
    pub async fn agent_apps_skills_inventory(&self, scope: &SkillReposQuery, token: &str)
        -> Result<SkillInventoryResponse, SkillInventoryError> {
        self.read_skill_inventory(scope, token, None).await
    }

    pub async fn agent_apps_skills_detail(&self, scope: &SkillReposQuery, id: &str, token: &str)
        -> Result<InstalledSkill, SkillInventoryError> {
        if id.is_empty() || id.len() > 255 || id.starts_with('.')
            || id.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')) {
            return Err(SkillInventoryError::InvalidId);
        }
        self.read_skill_inventory(scope, token, Some(id)).await
    }

    async fn read_skill_inventory<T: serde::de::DeserializeOwned>(&self,
        scope: &SkillReposQuery, token: &str, id: Option<&str>,
    ) -> Result<T, SkillInventoryError> {
        let mut url = reqwest::Url::parse(&format!("{}/v1/agent-apps/skills", self.endpoint.base_url()))
            .map_err(|_| SkillInventoryError::Transport)?;
        if let Some(id) = id {
            url.path_segments_mut().map_err(|_| SkillInventoryError::Transport)?.push(id);
        }
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillInventoryError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| SkillInventoryError::Transport)?
            .get(url).query(scope).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillInventoryError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillInventoryError::Http { status: status.as_u16() }); }
        response.json().await.map_err(|_| SkillInventoryError::Decode)
    }
}
