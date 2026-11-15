//! 原生工作流工具权限与结果处理。
use crate::{check_allowed, WorkflowToolRequest, WorkflowToolResult};
use serde_json::Value;
use tool_types::ToolCallResult;

pub trait NativeToolHost: Send + Sync {
    fn allowed_tools(&self) -> Option<&Value>;
    fn execute(&self, name: &str, args: &str) -> Result<ToolCallResult, String>;
}

pub fn call_tool(host: &impl NativeToolHost, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        if request.provider != "shiju"
            || request.action.is_some()
            || request.credential_id.is_some()
        {
            return Err("原生工作流工具仅支持 shiju，不能覆盖 action 或 credential".into());
        }
        check_allowed(&request.tool_name, host.allowed_tools())?;
        let args = serde_json::to_string(&request.inputs).map_err(|e| e.to_string())?;
        let result = host.execute(&request.tool_name, &args)?;
        // Shell terminal snapshots remain protocol data even on failure.
        // A workflow must still stop before downstream side effects.
        if !result.is_success() {
            return Err(result.error_text().unwrap_or_else(|| "workflow tool failed".into()));
        }
        Ok(WorkflowToolResult {
            text: Some(result.model_text()),
            json: Some(result.data.clone()),
            result: result.data,
            files: vec![],
        })
}
