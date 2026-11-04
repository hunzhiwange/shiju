use std::path::PathBuf;
use vw_api_types::id::TenantScope;
/// Parsed client input; never interpreted by a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoEditCommand {
    pub video_path: PathBuf,
    pub requirements: Option<String>,
}
/// Validated native input. Credentials remain in the runtime context.
#[derive(Debug, Clone)]
pub struct VideoEditRequest {
    pub video_path: PathBuf,
    pub original_request: String,
    pub requirements: Option<String>,
    pub tenant_scope: TenantScope,
    pub project_root: Option<PathBuf>,
}
