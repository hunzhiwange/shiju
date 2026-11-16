pub use workflow_video_edit::cloud::*;
use crate::skills::Skill;
use crate::tools::ToolRuntimeContext;
// 工作流 trait 实现已归属 workflow-engine。
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
pub(super) fn visible_shiju_root(skills: &[Skill]) -> Result<PathBuf, String> {
    workflow_video_edit::visible_instance::visible_shiju_root(
        skills.iter().filter(|skill| skill.name == "shiju").map(|skill| skill.location.as_deref()),
    )
}


pub async fn discover(context: ToolRuntimeContext, skills: &[Skill], allowed_tools: Option<Value>) -> Result<ShijuCloudVideoToolProvider, String> {
    let hub_root = visible_shiju_root(skills)?;
    ShijuCloudVideoToolProvider::discover(Arc::new(super::prepare::NativeVideoContext(context)), hub_root, allowed_tools).await
}
// WorkflowToolProvider 的实现由 workflow-engine 的 trait 所有者提供。
