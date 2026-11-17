//! Native slash entry. State belongs to one immutable tenant/template run.
use crate::{
    global_database::GlobalDatabase, skills::Skill, tools::ToolRuntimeContext, workflow::*,
};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use workflow_video_edit::entry::command_query;
#[cfg(any(test, target_arch = "wasm32"))]
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
        &super::prepare::NativeVideoContext(context.clone()),
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

// WorkflowToolProvider 的实现由 workflow-engine 的 trait 所有者提供。

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
