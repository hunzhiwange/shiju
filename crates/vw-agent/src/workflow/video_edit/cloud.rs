pub use workflow_video_edit::cloud::*;
use crate::skills::Skill;
use crate::tools::ToolRuntimeContext;
use crate::workflow::{WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
pub(super) fn visible_shiju_root(skills: &[Skill]) -> Result<PathBuf, String> {
    let mut matching = skills.iter().filter(|skill| skill.name == "shiju");
    let skill = matching.next().ok_or("当前会话没有授权的 shiju 技能")?;
    if matching.next().is_some() {
        return Err("当前会话有多个 shiju 实例，请先明确选择".into());
    }
    let location = skill.location.as_ref().ok_or("shiju 技能缺少客户端来源路径")?;
    let file = location.canonicalize().map_err(|_| "shiju 技能路径不可用")?;
    if !file.is_file() || file.file_name().and_then(|s| s.to_str()) != Some("SKILL.md") {
        return Err("shiju 来源必须是当前加载的 SKILL.md".into());
    }
    Ok(file.parent().ok_or("shiju 技能缺少包目录")?.to_path_buf())
}


pub async fn discover(context: ToolRuntimeContext, skills: &[Skill], allowed_tools: Option<Value>) -> Result<ShijuCloudVideoToolProvider, String> {
    let hub_root = visible_shiju_root(skills)?;
    ShijuCloudVideoToolProvider::discover(Arc::new(context), hub_root, allowed_tools).await
}
#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuCloudVideoToolProvider {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        let result = self.call_read(&request.provider, request.action.as_deref(), request.credential_id.as_deref(), &request.tool_name, &request.inputs).await?;
        Ok(WorkflowToolResult { result: result.clone(), json: Some(result), text: None, files: vec![] })
    }
}
