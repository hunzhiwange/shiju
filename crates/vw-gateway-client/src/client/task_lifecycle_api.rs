use vw_api_task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

use super::GatewayClient;

impl GatewayClient {
    /// 列出任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_list(
        &self,
    ) -> Result<Vec<TaskLifecycleWorkflowDto>, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_list()
            .await
    }

    /// 读取任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_get(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_get(id)
            .await
    }

    /// 新增任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_create(
        &self,
        body: &TaskLifecycleWorkflowUpsertBody,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_create(body)
            .await
    }

    /// 更新任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_update(
        &self,
        id: &str,
        body: &TaskLifecycleWorkflowUpsertBody,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_update(id, body)
            .await
    }

    /// 删除任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_delete(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDeleteResponse, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_delete(id)
            .await
    }

    /// 重置内置任务生命周期 workflow 为默认值。
    pub async fn task_lifecycle_workflows_reset(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        gateway_task_lifecycle::GatewayTaskLifecycleClient::new(self.transport().clone())
            .task_lifecycle_workflows_reset(id)
            .await
    }
}
