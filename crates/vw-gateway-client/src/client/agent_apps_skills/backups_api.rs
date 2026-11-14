use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::{backups_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillBackupError { Transport, Decode, InvalidId, Http { status: u16 } }
impl std::fmt::Display for SkillBackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId => f.write_str("invalid backup identity"),
            Self::Transport => f.write_str("skill backup transport failed"),
            Self::Decode => f.write_str("skill backup response invalid"),
            Self::Http { status } => write!(f, "skill backup HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillBackupError {}
impl GatewayClient {
    pub async fn agent_apps_skills_backups(&self, scope: &SkillReposQuery, token: &str)
        -> Result<SkillBackupsResponse, SkillBackupError> {
        self.skill_backup_request(reqwest::Method::GET, "backups", scope, token).await
    }
    pub async fn agent_apps_skills_backup_delete(&self, id: &str, request: &DeleteSkillBackupRequest, token: &str)
        -> Result<DeleteSkillBackupResponse, SkillBackupError> {
        validate_id(id)?;
        self.skill_backup_request(reqwest::Method::DELETE, &format!("backups/{id}"), request, token).await
    }
    pub async fn agent_apps_skills_backup_restore(&self, id: &str, request: &RestoreSkillBackupRequest, token: &str)
        -> Result<RestoreSkillBackupResponse, SkillBackupError> {
        validate_id(id)?;
        self.skill_backup_request(reqwest::Method::POST, &format!("backups/{id}/restore"), request, token).await
    }
    async fn skill_backup_request<T: serde::de::DeserializeOwned>(&self, method: reqwest::Method,
        path: &str, payload: &impl serde::Serialize, token: &str,
    ) -> Result<T, SkillBackupError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| SkillBackupError::Transport)?;
        credential.set_sensitive(true);
        let get = method == reqwest::Method::GET;
        let request = self.native_transport().map_err(|_| SkillBackupError::Transport)?
            .request(method, format!("{}/v1/agent-apps/skills/{path}", self.endpoint.base_url()))
            .header("x-agent-apps-token", credential);
        let request = if get { request.query(payload) } else { request.json(payload) };
        let response = crate::http::apply_request_context(request, &self.endpoint).send().await
            .map_err(|_| SkillBackupError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillBackupError::Http { status: status.as_u16() }); }
        response.json().await.map_err(|_| SkillBackupError::Decode)
    }
}

fn validate_id(id: &str) -> Result<(), SkillBackupError> {
    if !id.starts_with("skill-uninstall-") || id.len() > 255
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(SkillBackupError::InvalidId);
    }
    Ok(())
}
