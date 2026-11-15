//! 宿主技能目录注入与文件权限检查的兼容入口。
pub use tool_files::external_directory::{Kind, Options};
use crate::security::SecurityPolicy;
pub async fn assert_external_directory(security: &SecurityPolicy, target: Option<&str>, options: Option<Options>) -> Result<(), String> {
    tool_files::external_directory::assert_external_directory::<super::context::AgentContextHost>(security, target, options).await
}
#[cfg(test)]
use tool_files::external_directory::{normalize_path, resolve_full_path};
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
