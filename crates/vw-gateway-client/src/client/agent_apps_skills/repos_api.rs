use super::super::GatewayClient;
use vw_api_app_skills::repos_api::*;

impl GatewayClient {
    pub async fn skill_repos_list(&self, scope: &SkillReposQuery) -> Result<SkillReposResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .skill_repos_list(scope).await
    }
    pub async fn skill_repo_save(&self, request: &SaveSkillRepoRequest) -> Result<SkillReposResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .skill_repo_save(request).await
    }
    pub async fn skill_repo_remove(&self, owner: &str, name: &str,
        request: &RemoveSkillRepoRequest,
    ) -> Result<SkillReposResponse, String> {
        gateway_app_skills::GatewayAppSkillsClient::new(self.transport.clone())
            .skill_repo_remove(owner, name, request).await
    }
}
