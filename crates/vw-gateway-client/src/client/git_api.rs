use vw_api_git::{
    GitCommandRequest, GitCommandResponse, GitCommitRequest, GitCommitResponse, GitMergeRequest,
    GitMergeResponse,
};

use super::GatewayClient;

impl GatewayClient {
    pub async fn git_repository(
        &self,
        request: &vw_api_git::GitRepositoryRequest,
    ) -> Result<vw_api_git::GitRepositoryDto, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_repository(request)
            .await
    }
    pub async fn git_history_detail(
        &self,
        request: &vw_api_git::GitHistoryDetailRequest,
    ) -> Result<vw_api_git::GitHistoryDetailDto, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_history_detail(request)
            .await
    }
    pub async fn git_remote(
        &self,
        request: &vw_api_git::GitRemoteRequest,
    ) -> Result<vw_api_git::GitRemoteResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_remote(request)
            .await
    }

    /// 通过网关触发一次 Git 提交。
    pub async fn git_commit(
        &self,
        request: &GitCommitRequest,
    ) -> Result<GitCommitResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_commit(request)
            .await
    }

    /// 通过网关执行一次受限 Git 命令。
    pub async fn git_command(
        &self,
        request: &GitCommandRequest,
    ) -> Result<GitCommandResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_command(request)
            .await
    }

    /// 通过网关触发一次 Git 分支合并。
    pub async fn git_merge(&self, request: &GitMergeRequest) -> Result<GitMergeResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .git_merge(request)
            .await
    }
}
