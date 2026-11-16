use super::super::{host::*, options, skill_mounts::MountedSkills};
use super::super as runtime;
use agent_config::schema::{Config, AcpAgentConfig};
use model_types as provider;
use crate::Error;
use std::{path::Path, sync::Arc};
use super::super::model_source::profile::AcpLaunchProfile;
pub type ParsedAcpOptions = options::ParsedAcpOptions<MountedSkills>;
pub(crate) struct CacheHost;
tokio::task_local! { static REQUEST_CONFIG: Config; }
#[allow(unused_variables)]
impl Host for CacheHost {
    type Skills = MountedSkills;
    type TurnContext = ();
    type Approval<'a> = runtime::approval::PromptApproval<'a>;
    fn tenant() -> vw_api_transport::TenantScope { vw_api_transport::TenantScope::default() }
    fn tool_context() -> Option<Self::TurnContext> { None }
    fn desktop_context(context: &Self::TurnContext) -> Option<DesktopContext> { unimplemented!("unused host port in cache fixture") }
    fn worktree_lease(cwd: &Path) -> anyhow::Result<Option<Arc<std::fs::File>>> { Ok(None) }
    fn open_selection(session: &str, cwd: &str) -> anyhow::Result<runtime::lifecycle::Store> {
        static ROOT: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
        let root = ROOT.get_or_init(|| tempfile::tempdir().unwrap());
        let database = agent_database::global_database::GlobalDatabase::for_home(root.path())?;
        let store = app_settings_store::settings_store::GatewaySettingsStore::new(database)?;
        Ok(runtime::lifecycle::Store::open(store, Self::tenant(), session, cwd))
    }
    fn fork_replay<'a>(session: &'a str, cwd: &'a str) -> HostFuture<'a, anyhow::Result<Option<runtime::fork_replay::ForkReplay>>> { Box::pin(async { Ok(None) }) }
    fn model_source(options: &serde_json::Value) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String> { crate::prompt::acp_model_source(options) }
    fn config() -> HostFuture<'static, anyhow::Result<Config>> { Box::pin(async { Ok(REQUEST_CONFIG.try_with(Clone::clone).unwrap_or_default()) }) }
    fn project_id() -> Option<String> { unimplemented!("unused host port in cache fixture") }
    fn bind_agent<'a>(session: &'a str, cfg: &'a mut Config) -> HostFuture<'a, anyhow::Result<()>> { Box::pin(async { Ok(()) }) }
    fn agent_defaults<'a>(cfg: &'a Config, options: &'a serde_json::Value, agent: &'a str, cwd: &'a Path, parsed: &'a mut options::ParsedAcpOptions<Self::Skills>, messages: &'a serde_json::Value) -> HostFuture<'a, Result<serde_json::Value, Error>> { Box::pin(async move { assert!(options.get("agent").is_none(), "fixture has no bound agent defaults"); Ok(messages.clone()) }) }
    fn prepare_skills(mounts: &[acp_types::SessionSkillMount], cwd: &Path, session: &str, cfg: &Config) -> Result<Option<Self::Skills>, Error> { MountedSkills::prepare(mounts, cwd, session, &cfg.skills.security, &Self::tenant()) }
    fn reject_expanded(agent: &str, messages: &serde_json::Value) -> Result<(), Error> { runtime::skill_mounts::reject_expanded_invocations(agent, messages) }
    fn scanner(agent: &str, cfg: &mut AcpAgentConfig, skills: Option<&Self::Skills>) -> Result<(), Error> { runtime::skill_mounts::configure_native_scanner(agent, cfg, skills) }
    fn supplier(agent: &str, skills: Option<&Self::Skills>) -> Result<(), Error> { runtime::skill_mounts::prepare_supplier(agent, skills) }
    fn validate_mounts(agent: &str, selected: bool) -> Result<(), Error> { runtime::skill_mounts::validate(agent, selected) }
    fn prepare_launch<'a>(agent: &'a str, cfg: &'a AcpAgentConfig, model: &'a provider::Model, session: &'a str) -> HostFuture<'a, anyhow::Result<AcpLaunchProfile>> { unimplemented!("unused host port in cache fixture") }
    fn approval<'a>(slot: &'a parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>, session: &str) -> Self::Approval<'a> { runtime::approval::PromptApproval::bind(slot, None, tokio_util::sync::CancellationToken::new()) }
    fn observe_rpc(direction: acp_types::AcpMessageDirection, message: &acp_types::AcpJsonRpcMessage) {  }
    fn tool_input_preview(arguments: &str) -> String { arguments.to_owned() }
    fn record_usage(provider: &str, model: &str, session: &str, agent: &str, request: &str, usage: vw_api_usage::UsageTokens) { }
}
pub fn acp_client_cache_key(agent: &str, cfg: &AcpAgentConfig, cwd: &Path, parsed: &ParsedAcpOptions) -> String { runtime::session::acp_client_cache_key::<CacheHost>(agent, cfg, cwd, parsed) }
pub fn build_cached_acp_client(agent: &str, cfg: &AcpAgentConfig, parsed: &ParsedAcpOptions) -> Arc<runtime::cache::CachedAcpClient> { runtime::session::build_cached_acp_client::<CacheHost>(agent, cfg, parsed) }
pub fn get_cached_acp_client(agent: &str, cfg: &AcpAgentConfig, cwd: &Path, parsed: &ParsedAcpOptions) -> runtime::session::CachedAcpClientEntry { runtime::session::get_cached_acp_client::<CacheHost>(agent, cfg, cwd, parsed) }
mod contracts;
mod content;
mod session;
mod request;
mod routing;
