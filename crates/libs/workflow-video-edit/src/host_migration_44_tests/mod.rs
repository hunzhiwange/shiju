//! Domain tests migrated from the native video workflow; native security stays in vw-agent.
use crate::{assets, budget, cloud_failure, manifest, planning, prepare, transcript};
use crate::merge_tests;
use crate::command::{parse_video_edit_command, prepare_video_edit_request};
use crate::types::VideoEditRequest;
use crate::context::VideoContext;
use workflow_template_contract::DEFAULT_YAML as DEFAULT_VIDEO_EDIT_YAML;
use crate::entry;
use std::path::{Path, PathBuf};

mod assets_tests;
mod command_tests;
mod entry_tests;
mod manifest_tests;
mod prepare_tests;
mod planning_tests;
mod transcript_tests;

/// A local workspace port for domain scenarios, never a native authorization substitute.
#[derive(Clone)]
struct TestContext {
    root: PathBuf,
}
impl TestContext {
    fn new(_session: impl Into<String>, root: Option<String>) -> Self {
        Self { root: PathBuf::from(root.expect("domain fixture requires workspace")) }
    }
}
#[async_trait::async_trait]
impl VideoContext for TestContext {
    fn question_tool_id(&self) -> &'static str { "ask_user_question" }
    fn workspace_root(&self) -> Option<PathBuf> { Some(self.root.clone()) }
    fn workspace_dir(&self) -> PathBuf { self.root.clone() }
    fn is_resolved_path_allowed(&self, path: &Path) -> bool {
        path.starts_with(self.root.canonicalize().expect("fixture workspace"))
    }
    fn abort_token(&self) -> Option<tokio_util::sync::CancellationToken> { None }
    fn tenant_scope(&self) -> vw_api_transport::TenantScope { Default::default() }
    fn execute_tool_call(&self, _: &str, _: &str) -> Result<tool_types::ToolCallResult, tool_context::ToolCallError> {
        panic!("domain fixture must not invoke native tools")
    }
    async fn choose_scope(&self, _: &[manifest::CloudVideoScope]) -> anyhow::Result<manifest::CloudVideoScope> {
        anyhow::bail!("domain fixture requires explicit scope")
    }
}
