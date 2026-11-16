//! Trusted host IO adapter for the ACP runtime.
use super::{Error, ParsedAcpOptions, StreamEvent};
use llm_runtime::acp::host::{Host, HostFuture, DesktopContext};
use crate::{config, provider::provider};
use agent_auth as auth;
use std::{path::Path, sync::Arc};
use serde_json::Value;

pub(super) struct RuntimeHost;
pub(super) fn desktop_context(context: &crate::tools::ToolUseContext) -> DesktopContext {
    DesktopContext { session: context.session().to_owned(), workspace: context.workspace_root(), full_access: context.full_access_enabled() }
}
impl Host for RuntimeHost {
    type Skills = super::skill_mounts::MountedSkills;
    type TurnContext = Arc<crate::tools::ToolUseContext>;
    type Approval<'a> = super::approval::PromptApproval<'a>;
    fn tenant() -> vw_api_transport::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn tool_context() -> Option<Self::TurnContext> { crate::tools::current_tool_use_context() }
    fn desktop_context(context: &Self::TurnContext) -> Option<DesktopContext> {
        (context.channel() == Some("desktop")).then(|| desktop_context(context))
    }
    fn worktree_lease(cwd: &Path) -> anyhow::Result<Option<Arc<std::fs::File>>> { Ok(crate::worktree::registry::use_lease(cwd)?.map(Arc::new)) }
    fn open_selection(session: &str, cwd: &str) -> anyhow::Result<llm_runtime::acp::lifecycle::Store> { super::lifecycle::Store::open(session, cwd) }
    fn fork_replay<'a>(session: &'a str, cwd: &'a str) -> HostFuture<'a, anyhow::Result<Option<llm_runtime::acp::fork_replay::ForkReplay>>> {
        Box::pin(async move { Ok(super::fork_replay::ForkReplay::load(session, cwd).await?.map(super::fork_replay::ForkReplay::into_runtime)) })
    }
    fn model_source(options: &Value) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String> { super::super::prompt::acp_model_source(options) }
    fn config() -> HostFuture<'static, anyhow::Result<config::schema::Config>> { Box::pin(async { Ok(config::get_current().await?) }) }
    fn project_id() -> Option<String> { crate::project::instance::project().map(|project| project.id) }
    fn bind_agent<'a>(session: &'a str, cfg: &'a mut config::schema::Config) -> HostFuture<'a, anyhow::Result<()>> { Box::pin(crate::session::agent_binding::apply_to_config(session, cfg)) }
    fn agent_defaults<'a>(cfg: &'a config::schema::Config, options: &'a Value, agent: &'a str, cwd: &'a Path, parsed: &'a mut ParsedAcpOptions, messages: &'a Value) -> HostFuture<'a, Result<Value, Error>> { Box::pin(super::config::apply_agent_defaults(cfg, options, agent, cwd, parsed, messages)) }
    fn prepare_skills(mounts: &[acp_types::SessionSkillMount], cwd: &Path, session: &str, cfg: &config::schema::Config) -> Result<Option<Self::Skills>, Error> { Self::Skills::prepare(mounts, cwd, session, &cfg.skills.security) }
    fn reject_expanded(agent: &str, messages: &Value) -> Result<(), Error> { super::skill_mounts::reject_expanded_invocations(agent, messages) }
    fn scanner(agent: &str, cfg: &mut config::schema::AcpAgentConfig, skills: Option<&Self::Skills>) -> Result<(), Error> { super::skill_mounts::configure_native_scanner(agent, cfg, skills) }
    fn supplier(agent: &str, skills: Option<&Self::Skills>) -> Result<(), Error> { super::skill_mounts::prepare_supplier(agent, skills) }
    fn validate_mounts(agent: &str, selected: bool) -> Result<(), Error> { super::skill_mounts::validate(agent, selected) }
    fn prepare_launch<'a>(agent: &'a str, cfg: &'a config::schema::AcpAgentConfig, model: &'a provider::Model, session: &'a str) -> HostFuture<'a, anyhow::Result<super::model_source::profile::AcpLaunchProfile>> { Box::pin(super::model_source::prepare(agent, cfg, model, session)) }
    fn approval<'a>(slot: &'a parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>, session: &str) -> Self::Approval<'a> { super::approval::PromptApproval::bind(slot, session) }
    fn observe_rpc(_direction: acp_types::AcpMessageDirection, _message: &acp_types::AcpJsonRpcMessage) {
        #[cfg(test)]
        super::model_source::acceptance_tests::observe_rpc(_direction, _message);
    }
    fn tool_input_preview(arguments: &str) -> String { crate::util::truncate_with_ellipsis(&crate::agent::loop_::scrub_credentials(arguments), 200) }
    fn record_usage(provider: &str, model: &str, session: &str, agent: &str, id: &str, usage: vw_api_usage::UsageTokens) {
        if let Some(mut request) = crate::usage_bridge::native::NativeRequest::begin(provider, model, Some(session)) {
            request.native_identity(agent, id);
            request.complete(Some(usage));
        }
    }
}
pub(crate) async fn do_stream_request_acp(model: &provider::Model, info: &provider::Info, auth: Option<&auth::Info>, options: &Value, messages: &Value, session: &str, abort: Option<&tokio::sync::watch::Receiver<bool>>, on_event: &mut impl FnMut(StreamEvent)) -> Result<(), Error> {
    llm_runtime::acp::request::do_stream_request_acp::<RuntimeHost>(model, info, auth, options, messages, session, abort, on_event).await
}
pub(super) use llm_runtime::acp::request::validate_session_model;
#[cfg(test)]
use llm_runtime::acp::request::apply_default_mode;
#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
