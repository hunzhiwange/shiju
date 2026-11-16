pub use workflow_video_edit::prepare::*;
use crate::tools::ToolRuntimeContext;
use super::manifest::CloudVideoScope;
use anyhow::{Context, Result, bail, ensure};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use workflow_video_edit::context::VideoContext;

#[derive(Clone)]
pub(crate) struct NativeVideoContext(pub ToolRuntimeContext);
impl std::ops::Deref for NativeVideoContext {
    type Target = ToolRuntimeContext;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[async_trait::async_trait]
impl VideoContext for NativeVideoContext {
    fn question_tool_id(&self) -> &'static str { crate::tools::ASK_USER_QUESTION_TOOL_ID }
    fn workspace_root(&self) -> Option<PathBuf> { self.tool_use_context().workspace_root() }
    fn workspace_dir(&self) -> PathBuf { self.path_security_policy().workspace_dir.clone() }
    fn is_resolved_path_allowed(&self, path: &Path) -> bool { self.path_security_policy().is_resolved_path_allowed(path) }
    fn abort_token(&self) -> Option<CancellationToken> { self.tool_use_context().abort_token().cloned() }
    fn tenant_scope(&self) -> vw_api_transport::TenantScope { self.tool_use_context().tenant_scope() }
    fn execute_tool_call(&self, name: &str, input: &str) -> Result<tool_types::ToolCallResult, tool_context::ToolCallError> {
        crate::tools::execute_tool_call(name, input, self)
    }
    async fn choose_scope(&self, scopes: &[CloudVideoScope]) -> Result<CloudVideoScope> {
            let labels = scopes
                .iter()
                .map(|s| {
                    format!(
                        "brand={} store={}",
                        s.brand_id.as_deref().unwrap_or("-"),
                        s.store_id.as_deref().unwrap_or("-")
                    )
                })
                .collect::<Vec<_>>();
            let input = crate::question::AskInput {
                session_id: self.session.clone(),
                tool: None,
                questions: vec![crate::question::Info {
                    question: "请选择本次视频的云端归属".into(),
                    header: "云端归属".into(),
                    options: labels
                        .iter()
                        .map(|label| crate::question::OptionInfo {
                            label: label.clone(),
                            description: "使用该已授权空间准备工程与原片".into(),
                            preview: None,
                        })
                        .collect(),
                    multiple: Some(false),
                    custom: Some(false),
                }],
            };
            let abort = self.abort_token().unwrap_or_default();
            let answers = tokio::select! {
                result = crate::session::ui_store::with_tenant_scope(self.tenant_scope(), crate::question::ask(input)) => result.context("cloud space choice cancelled")?,
                _ = abort.cancelled() => bail!("cancelled"),
            };
            ensure!(answers.len() == 1 && answers[0].len() == 1, "invalid cloud scope answer");
            let index = labels
                .iter()
                .position(|l| *l == answers[0][0])
                .context("unauthorized cloud scope answer")?;
            Ok(scopes[index].clone())
    }
}
