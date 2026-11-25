//! Trusted host IO used at the original request sequence points.
use std::{collections::HashMap, future::Future, pin::Pin, path::{Path, PathBuf}, sync::Arc};
use agent_config::schema::{Config, AcpAgentConfig};
use model_types as provider;
use crate::Error;
use super::{options::ParsedAcpOptions, model_source::profile::AcpLaunchProfile};
pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait SkillLease: Clone + Send + Sync + 'static {
    fn fingerprint(&self) -> &str;
    fn root(&self) -> &Path;
    fn validate(&self) -> Result<(), Error>;
    fn snapshot(&self, _destination: &Path) -> Result<(), Error> {
        Err(super::acp_option_error("The host cannot snapshot this skill lease"))
    }
}

pub struct DesktopContext {
    pub session: String,
    pub workspace: Option<PathBuf>,
    pub full_access: bool,
}

/// Request-only data supplied by the host after checking the server-owned binding.
#[derive(Clone)]
pub struct EmployeeAcpLaunch {
    pub employee_id: String,
    pub kind: EmployeeAcpKind,
    pub account_scope: String,
    pub state_dir: PathBuf,
    pub credential: String,
    pub stop: Option<tokio_util::sync::CancellationToken>,
}

#[derive(Clone, Copy)]
pub enum EmployeeAcpKind {
    Private,
    Shared,
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
    /// Install the bounded workbench connection for this authenticated turn, if any.
    fn apply_workbench_child_environment(
        _session: &str,
        _cwd: &Path,
        _env: &mut HashMap<String, String>,
    ) {
    }
    /// Return launch data only for this turn's authenticated employee binding.
    /// The gateway carries it across the stream worker boundary outside JSON options.
    fn employee_acp_launch(_session: &str) -> Option<EmployeeAcpLaunch> {
        None
    }
    fn employee_acp_executable() -> std::io::Result<PathBuf> {
        acp_agent_registry::employee_acp_executable()
    }
    fn bind_agent<'a>(session: &'a str, cfg: &'a mut Config) -> HostFuture<'a, anyhow::Result<()>>;
    fn agent_defaults<'a>(cfg: &'a Config, options: &'a serde_json::Value, agent: &'a str, cwd: &'a Path, parsed: &'a mut ParsedAcpOptions<Self::Skills>, messages: &'a serde_json::Value) -> HostFuture<'a, Result<serde_json::Value, Error>>;
    fn prepare_skills(mounts: &[acp_types::SessionSkillMount], cwd: &Path, session: &str, cfg: &Config) -> Result<Option<Self::Skills>, Error>;
    fn prepare_reviewed_skills<'a>(
        mounts: &'a mut Vec<acp_types::SessionSkillMount>,
        cwd: &'a Path,
        _session: &'a str,
        scoped_session: &'a str,
        cfg: &'a Config,
    ) -> HostFuture<'a, Result<Option<Self::Skills>, Error>> {
        Box::pin(async move { Self::prepare_skills(mounts, cwd, scoped_session, cfg) })
    }
    fn reject_expanded(agent: &str, messages: &serde_json::Value) -> Result<(), Error>;
    fn scanner(agent: &str, cfg: &mut AcpAgentConfig, skills: Option<&Self::Skills>) -> Result<(), Error>;
    fn supplier(agent: &str, skills: Option<&Self::Skills>) -> Result<(), Error>;
    fn validate_mounts(agent: &str, selected: bool) -> Result<(), Error>;
    fn prepare_launch<'a>(agent: &'a str, cfg: &'a AcpAgentConfig, model: &'a provider::Model, session: &'a str) -> HostFuture<'a, anyhow::Result<AcpLaunchProfile>>;
    fn prepare_stdio_mcp_launch<'a>(
        _agent: &'a str,
        _config: &'a AcpAgentConfig,
        _model: &'a provider::Model,
        _session: &'a str,
        _cwd: &'a Path,
        _servers: &'a [acp_types::AcpMcpServer],
    ) -> HostFuture<'a, anyhow::Result<AcpLaunchProfile>> {
        Box::pin(async { anyhow::bail!("Managed stdio MCP bridge is unavailable") })
    }

    fn prepare_openclaw_mcp_launch<'a>(
        config: &'a AcpAgentConfig,
        model: &'a provider::Model,
        session: &'a str,
        mcp: &'a serde_json::Map<String, serde_json::Value>,
    ) -> HostFuture<'a, anyhow::Result<AcpLaunchProfile>>;

    fn approval<'a>(slot: &'a parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>, session: &str) -> Self::Approval<'a>;
    fn observe_rpc(direction: acp_types::AcpMessageDirection, message: &acp_types::AcpJsonRpcMessage);
    fn tool_input_preview(arguments: &str) -> String;
    fn record_usage(provider: &str, model: &str, session: &str, agent: &str, request: &str, usage: vw_api_usage::UsageTokens);
}
