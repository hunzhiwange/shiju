use super::super::GatewayClient;
use {
    vw_api_app_skills::jobs_api::SkillJobResponse,
    vw_api_app_skills::repos_api::SkillReposQuery,
    vw_api_transport::id::TaskId,
};
pub use gateway_app_skills::{SkillJobError};

impl GatewayClient {
    pub async fn agent_apps_skills_job(&self, scope: &SkillReposQuery, id: &TaskId, token: &str) -> Result<SkillJobResponse, SkillJobError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_job(scope, id, token).await
    }
    pub async fn agent_apps_skills_cancel_job(&self, scope: &SkillReposQuery, id: &TaskId, token: &str) -> Result<SkillJobResponse, SkillJobError> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .agent_apps_skills_cancel_job(scope, id, token).await
    }
}
