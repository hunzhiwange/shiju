use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::{storage_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStorageError { Transport, Decode, Http { status: u16 } }
impl std::fmt::Display for SkillStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill storage transport failed"),
            Self::Decode => f.write_str("skill storage response invalid"),
            Self::Http { status } => write!(f, "skill storage HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillStorageError {}
impl GatewayClient {
    pub async fn agent_apps_skills_storage(&self, scope: &SkillReposQuery, token: &str)
        -> Result<SkillStorageResponse, SkillStorageError> {
        self.skill_storage_request(reqwest::Method::GET, "storage", scope, token).await
    }
    pub async fn agent_apps_skills_storage_patch(&self, request: &SkillStoragePatch, token: &str)
        -> Result<SkillStorageResponse, SkillStorageError> {
        self.skill_storage_request(reqwest::Method::PATCH, "storage", request, token).await
    }
    pub async fn agent_apps_skills_storage_move(&self, request: &SkillStorageMoveRequest, token: &str)
        -> Result<SkillStorageMoveResponse, SkillStorageError> {
        self.skill_storage_request(reqwest::Method::POST, "storage/move", request, token).await
    }
    async fn skill_storage_request<T: serde::de::DeserializeOwned>(&self, method: reqwest::Method,
        path: &str, payload: &impl serde::Serialize, token: &str,
    ) -> Result<T, SkillStorageError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| SkillStorageError::Transport)?;
        credential.set_sensitive(true);
        let get = method == reqwest::Method::GET;
        let request = self.native_transport().map_err(|_| SkillStorageError::Transport)?
            .request(method, format!("{}/v1/agent-apps/skills/{path}", self.endpoint.base_url()))
            .header("x-agent-apps-token", credential);
        let request = if get { request.query(payload) } else { request.json(payload) };
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await
            .map_err(|_| SkillStorageError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillStorageError::Http { status: status.as_u16() }); }
        response.json().await.map_err(|_| SkillStorageError::Decode)
    }
}
