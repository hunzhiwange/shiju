use vw_api_types::task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

use super::GatewayClient;

const TASK_LIFECYCLE_WORKFLOWS_PATH: &str = "/v1/task-lifecycle/workflows";

impl GatewayClient {
    /// 列出任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_list(
        &self,
    ) -> Result<Vec<TaskLifecycleWorkflowDto>, String> {
        self.get_json(TASK_LIFECYCLE_WORKFLOWS_PATH, &[]).await
    }

    /// 读取任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_get(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        self.get_json(&task_lifecycle_workflow_path(id)?, &[]).await
    }

    /// 新增任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_create(
        &self,
        body: &TaskLifecycleWorkflowUpsertBody,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        self.post_json(TASK_LIFECYCLE_WORKFLOWS_PATH, &[], body).await
    }

    /// 更新任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_update(
        &self,
        id: &str,
        body: &TaskLifecycleWorkflowUpsertBody,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        self.put_json(&task_lifecycle_workflow_path(id)?, &[], body).await
    }

    /// 删除任务生命周期 workflow。
    pub async fn task_lifecycle_workflows_delete(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDeleteResponse, String> {
        self.delete_json(&task_lifecycle_workflow_path(id)?, &[], &serde_json::json!({})).await
    }

    /// 重置内置任务生命周期 workflow 为默认值。
    pub async fn task_lifecycle_workflows_reset(
        &self,
        id: &str,
    ) -> Result<TaskLifecycleWorkflowDto, String> {
        self.post_json(&task_lifecycle_workflow_reset_path(id)?, &[], &serde_json::json!({})).await
    }
}

fn task_lifecycle_workflow_path(id: &str) -> Result<String, String> {
    if id.trim().is_empty() {
        return Err("task lifecycle workflow id must not be empty".to_string());
    }
    Ok(format!("{TASK_LIFECYCLE_WORKFLOWS_PATH}/{}", encode_path_segment(id)))
}

fn task_lifecycle_workflow_reset_path(id: &str) -> Result<String, String> {
    Ok(format!("{}/reset", task_lifecycle_workflow_path(id)?))
}

fn encode_path_segment(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    })
}

#[cfg(test)]
#[path = "task_lifecycle_api_tests.rs"]
mod task_lifecycle_api_tests;
