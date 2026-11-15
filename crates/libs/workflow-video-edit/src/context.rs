//! Native execution and user interaction supplied by the application host.
use std::{path::{Path, PathBuf}, sync::Arc};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
use tool_context::ToolCallError;
use tool_types::ToolCallResult;
use crate::manifest::CloudVideoScope;

#[async_trait::async_trait]
pub trait VideoContext: Send + Sync {
    fn question_tool_id(&self) -> &'static str;
    fn workspace_root(&self) -> Option<PathBuf>;
    fn workspace_dir(&self) -> PathBuf;
    fn is_resolved_path_allowed(&self, path: &Path) -> bool;
    fn abort_token(&self) -> Option<CancellationToken>;
    fn tenant_scope(&self) -> TenantScope;
    fn execute_tool_call(&self, name: &str, input: &str) -> Result<ToolCallResult, ToolCallError>;
    async fn choose_scope(&self, scopes: &[CloudVideoScope]) -> anyhow::Result<CloudVideoScope>;
}
pub type VideoContextHandle = Arc<dyn VideoContext>;
