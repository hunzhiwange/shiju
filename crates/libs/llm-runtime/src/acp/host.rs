//! Trusted host IO used at the original request sequence points.
use std::{future::Future, pin::Pin, path::{Path, PathBuf}, sync::Arc};
use agent_config::schema::{Config, AcpAgentConfig};
use model_types as provider;
use crate::Error;
use super::{options::ParsedAcpOptions, model_source::profile::AcpLaunchProfile};
pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait SkillLease: Clone + Send + Sync + 'static {
    fn fingerprint(&self) -> &str;
    fn root(&self) -> &Path;
    fn validate(&self) -> Result<(), Error>;
}

pub struct DesktopContext {
    pub session: String,
    pub workspace: Option<PathBuf>,
    pub full_access: bool,
}

pub trait Host: Send + Sync + 'static {
    type Skills: SkillLease;
    type TurnContext: Send + Sync;
    type Approval<'a>: Send where Self: 'a;
    fn tenant() -> vw_api_transport::TenantScope;
    fn tool_context() -> Option<Self::TurnContext>;
    fn desktop_context(context: &Self::TurnContext) -> Option<DesktopContext>;
    fn worktree_lease(cwd: &Path) -> anyhow::Result<Option<Arc<std::fs::File>>>;
    fn open_selection(session: &str, cwd: &str) -> anyhow::Result<super::lifecycle::Store>;
    fn fork_replay<'a>(session: &'a str, cwd: &'a str) -> HostFuture<'a, anyhow::Result<Option<super::fork_replay::ForkReplay>>>;
    fn model_source(options: &serde_json::Value) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String>;
    fn config() -> HostFuture<'static, anyhow::Result<Config>>;
    fn project_id() -> Option<String>;
    fn bind_agent<'a>(session: &'a str, cfg: &'a mut Config) -> HostFuture<'a, anyhow::Result<()>>;
    fn agent_defaults<'a>(cfg: &'a Config, options: &'a serde_json::Value, agent: &'a str, cwd: &'a Path, parsed: &'a mut ParsedAcpOptions<Self::Skills>, messages: &'a serde_json::Value) -> HostFuture<'a, Result<serde_json::Value, Error>>;
    fn prepare_skills(mounts: &[acp_types::SessionSkillMount], cwd: &Path, session: &str, cfg: &Config) -> Result<Option<Self::Skills>, Error>;
    fn reject_expanded(agent: &str, messages: &serde_json::Value) -> Result<(), Error>;
    fn scanner(agent: &str, cfg: &mut AcpAgentConfig, skills: Option<&Self::Skills>) -> Result<(), Error>;
    fn supplier(agent: &str, skills: Option<&Self::Skills>) -> Result<(), Error>;
    fn validate_mounts(agent: &str, selected: bool) -> Result<(), Error>;
    fn prepare_launch<'a>(agent: &'a str, cfg: &'a AcpAgentConfig, model: &'a provider::Model, session: &'a str) -> HostFuture<'a, anyhow::Result<AcpLaunchProfile>>;
    fn approval<'a>(slot: &'a parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>, session: &str) -> Self::Approval<'a>;
    fn observe_rpc(direction: acp_types::AcpMessageDirection, message: &acp_types::AcpJsonRpcMessage);
    fn tool_input_preview(arguments: &str) -> String;
    fn record_usage(provider: &str, model: &str, session: &str, agent: &str, request: &str, usage: vw_api_usage::UsageTokens);
}
