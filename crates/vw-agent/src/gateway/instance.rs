//! Project instance host composition.
use gateway_types::ApiError;
use crate::project;
use std::path::PathBuf;
pub use agent_gateway::instance::{InstanceQuery, resolve_directory, normalize_rel_path, load_local_project};
#[cfg(test)]
use agent_gateway::instance::display_directory;
pub async fn with_instance<T>(directory: String, f: impl FnOnce() -> project::BoxFuture<Result<T, ApiError>> + Send + 'static) -> Result<T, ApiError> {
 agent_gateway::instance::with_instance(directory, |path| Box::pin(project::instance_bootstrap(path)), f).await
}
#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/instance_tests.rs"]
mod instance_tests;

#[cfg(test)]
fn instance_init() -> Box<dyn FnOnce() -> project::BoxFuture<()> + Send + 'static> { agent_gateway::instance::instance_init(|path| Box::pin(project::instance_bootstrap(path))) }
