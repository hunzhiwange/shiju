use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::import_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillImportError {
    Transport,
    Decode,
    Http { status: u16 },
}
impl std::fmt::Display for SkillImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill import transport failed"),
            Self::Decode => f.write_str("skill import response invalid"),
            Self::Http { status } => write!(f, "skill import HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillImportError {}
impl GatewayClient {
    pub async fn agent_apps_skills_import(&self, query: &ImportSkillRequest, token: &str)
        -> Result<ImportSkillResponse, SkillImportError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillImportError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| SkillImportError::Transport)?
            .post(format!("{}/v1/agent-apps/skills/import", self.endpoint.base_url()))
            .json(query).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillImportError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillImportError::Transport)?;
        if !status.is_success() {
            return Err(SkillImportError::Http { status: status.as_u16() });
        }
        serde_json::from_slice(&bytes).map_err(|_| SkillImportError::Decode)
    }
    pub async fn agent_apps_skills_unmanaged(&self, query: &UnmanagedSkillsQuery, token: &str)
        -> Result<UnmanagedSkillsResponse, SkillImportError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillImportError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| SkillImportError::Transport)?
            .get(format!("{}/v1/agent-apps/skills/unmanaged", self.endpoint.base_url()))
            .query(query).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillImportError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillImportError::Transport)?;
        if !status.is_success() {
            return Err(SkillImportError::Http { status: status.as_u16() });
        }
        serde_json::from_slice(&bytes).map_err(|_| SkillImportError::Decode)
    }
}
