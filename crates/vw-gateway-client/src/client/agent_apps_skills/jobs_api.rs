use super::super::GatewayClient;
use vw_api_types::{agent_apps_skills::{jobs_api::SkillJobResponse, repos_api::SkillReposQuery}, id::TaskId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillJobError { Transport, Decode, InvalidTaskId, Http { status: u16 } }
impl std::fmt::Display for SkillJobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill task transport failed"),
            Self::Decode => f.write_str("skill task response invalid"),
            Self::InvalidTaskId => f.write_str("invalid skill task ID"),
            Self::Http { status } => write!(f, "skill task HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillJobError {}
impl GatewayClient {
    pub async fn agent_apps_skills_job(&self, scope: &SkillReposQuery, id: &TaskId, token: &str)
        -> Result<SkillJobResponse, SkillJobError> {
        self.skill_job_request(scope, id, token, false).await
    }
    pub async fn agent_apps_skills_cancel_job(&self, scope: &SkillReposQuery, id: &TaskId, token: &str)
        -> Result<SkillJobResponse, SkillJobError> {
        self.skill_job_request(scope, id, token, true).await
    }
    async fn skill_job_request(&self, scope: &SkillReposQuery, id: &TaskId, token: &str, cancel: bool)
        -> Result<SkillJobResponse, SkillJobError> {
        let id: &str = id.as_ref();
        if id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) {
            return Err(SkillJobError::InvalidTaskId);
        }
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| SkillJobError::Transport)?;
        credential.set_sensitive(true);
        let client = self.native_transport().map_err(|_| SkillJobError::Transport)?;
        let url = format!("{}/v1/agent-apps/skills/jobs/{id}{}", self.endpoint.base_url(), if cancel { "/cancel" } else { "" });
        let request = if cancel { client.post(url) } else { client.get(url) };
        let request = request.query(scope).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillJobError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillJobError::Http { status: status.as_u16() }); }
        let body = response.bytes().await.map_err(|_| SkillJobError::Transport)?;
        let result: SkillJobResponse = serde_json::from_slice(&body).map_err(|_| SkillJobError::Decode)?;
        if result.scope != *scope || result.task_id.as_ref() != id { return Err(SkillJobError::Decode); }
        Ok(result)
    }
}
