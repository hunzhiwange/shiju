//! Native slash entry. State belongs to one immutable tenant/template run.
use crate::{
    global_database::GlobalDatabase, skills::Skill, tools::ToolRuntimeContext, workflow::*,
};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
pub use workflow_video_edit::entry::{command_query, VideoEntryCloud, ShijuVideoEntryProvider};
use workflow_video_edit::entry::{VideoEntryHost, VideoEntryInput, VideoEntryServices};

pub(crate) async fn build(
    request: &crate::session::processor::Request,
    context: &ToolRuntimeContext,
    skills: &[Skill],
) -> Result<(WorkflowRuntime, vw_api_workflow::WorkflowRunRequest)> {
    let built = workflow_video_edit::entry::build_entry(
        VideoEntryInput {
            query: &request.query,
            acp_test: request.options.get("acp_test").and_then(Value::as_bool) == Some(true),
            allowed_tools: request.options.get("allowed_tools").cloned(),
            model: request.model.clone(),
            abort_token: request.abort_token.clone(),
        },
        context,
        &NativeVideoEntryHost { #[cfg(test)] session: &request.session, context, skills },
    ).await?;
    let runtime = WorkflowRuntime {
        video_edit_budget: built.budget,
        provider: built.provider,
        model: built.model,
        temperature: 0.2,
        tool_provider: Some(Arc::new(built.tools)),
        agent_provider: None,
        knowledge_provider: None,
        document_extractor: None,
        pause_store: None,
    };
    Ok((runtime, built.request))
}

#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuVideoEntryProvider {
    async fn call(&self, request: WorkflowToolRequest) -> std::result::Result<WorkflowToolResult, String> {
        let (result, text) = self.execute(&request.provider, &request.tool_name, request.action.as_deref(), request.credential_id.as_deref(), &request.inputs).await.map_err(|e| format!("云端剪辑失败: {e:#}"))?;
        Ok(WorkflowToolResult { result: result.clone(), json: Some(result), text: Some(text), files: vec![] })
    }
}

struct NativeVideoEntryHost<'a> {
    #[cfg(test)]
    session: &'a str,
    context: &'a ToolRuntimeContext,
    skills: &'a [Skill],
}
#[async_trait::async_trait]
impl VideoEntryHost for NativeVideoEntryHost<'_> {
    fn database(&self) -> Result<GlobalDatabase> {
        GlobalDatabase::for_home(&crate::global::paths().home)
    }
    async fn services(&self, allowed_tools: Option<Value>) -> Result<VideoEntryServices> {
        #[cfg(test)]
        if let Some((cloud, provider)) = super::entry_test_transport::get(self.session, self.context) {
            return Ok(VideoEntryServices { cloud, provider });
        }
        let cloud = super::cloud::discover(self.context.clone(), self.skills, allowed_tools)
            .await.map_err(anyhow::Error::msg)?;
        Ok(VideoEntryServices {
            cloud: Arc::new(cloud),
            provider: Arc::new(crate::providers::session_llm::SessionLlmProvider::new()),
        })
    }
}
