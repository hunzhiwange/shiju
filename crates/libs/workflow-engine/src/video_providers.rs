//! 为视频库的真实提供方实现本库工作流执行契约。

use crate::{WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult};
use workflow_video_edit::{cloud::ShijuCloudVideoToolProvider, entry::ShijuVideoEntryProvider};

#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuVideoEntryProvider {
    async fn call(&self, request: WorkflowToolRequest) -> std::result::Result<WorkflowToolResult, String> {
        let (result, text) = self.execute(&request.provider, &request.tool_name, request.action.as_deref(), request.credential_id.as_deref(), &request.inputs).await.map_err(|e| format!("云端剪辑失败: {e:#}"))?;
        Ok(WorkflowToolResult { result: result.clone(), json: Some(result), text: Some(text), files: vec![] })
    }
}

#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuCloudVideoToolProvider {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        let result = self.call_read(&request.provider, request.action.as_deref(), request.credential_id.as_deref(), &request.tool_name, &request.inputs).await?;
        Ok(WorkflowToolResult { result: result.clone(), json: Some(result), text: None, files: vec![] })
    }
}
