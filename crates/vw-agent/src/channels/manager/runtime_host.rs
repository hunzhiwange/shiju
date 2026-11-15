//! 通道运行层到宿主会话、命令和持久化能力的真实适配。
use super::*;
use channel_runtime::host::{ChannelExecutionInput, ChannelHost, ScopedFuture};
pub(super) struct RuntimeHost;
pub(super) static HOST: RuntimeHost = RuntimeHost;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ChannelHost for RuntimeHost {
    fn now_ms(&self) -> u64 { crate::session::session::now_ms() }
    fn scrub_credentials(&self, text: &str) -> String { scrub_credentials(text) }
    fn draft_sentinels(&self) -> (&'static str, &'static str, &'static str) {
        use crate::agent::loop_::{DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL};
        (DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL)
    }
    async fn save_config(&self, config: &Config) -> Result<()> { save_config(config).await }
    async fn in_tenant_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) {
        crate::session::ui_store::with_tenant_scope(snapshot.scope.chat.tenant.clone(), operation).await;
    }
    async fn in_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) {
        let context = crate::project::instance::Context { directory: snapshot.directory.to_string_lossy().into_owned(), worktree: snapshot.project.worktree.clone(), project: snapshot.project.clone() };
        crate::session::ui_store::with_tenant_scope(snapshot.scope.chat.tenant.clone(), crate::project::instance::provide_context(context, operation)).await;
    }
    async fn runtime_command(&self, ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage, channel: Option<&Arc<dyn Channel>>) -> bool {
        handle_runtime_command_if_needed(ctx, msg, channel).await
    }
    async fn session_id(&self, snapshot: &ChannelMessageSnapshot, msg: &traits::ChannelMessage, route: Option<ChannelRouteSelection>) -> Result<String> {
        resolve_or_create_sender_session_id(snapshot, msg, route).await
    }
    async fn collect_response(&self, input: ChannelExecutionInput, delta: Option<tokio::sync::mpsc::Sender<String>>) -> Result<ChannelProcessorResponse> {
        collect_channel_response(crate::session::processor::Request {
            stream: input.stream, session: input.session, query: input.query,
            root: Some(input.directory.to_string_lossy().into_owned()), model: Some(input.model),
            reliability: input.reliability, options: input.options, approval: Some(input.approval),
            channel_name: Some(input.channel_name), non_cli_approval_context: Some(input.non_cli_approval_context),
            assistant_message_id: None, history: input.history, persist_app_session_artifacts: true,
            app_session_scope: Some(input.project), abort_token: Some(input.abort),
            session_tools: crate::session::runtime::SessionToolset::default(), session_skills: Default::default(),
        }, delta).await
    }
    fn reap_shell(&self, snapshot: &ChannelMessageSnapshot, session: &str) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(host) = crate::tools::shell::processes::ShellProcesses::existing() {
            let scope = crate::tools::shell::processes::ShellProcessScope::new(serde_json::to_string(&snapshot.scope.chat.tenant)?, &snapshot.directory, session.to_string())?;
            host.cancel_scope(&scope)?;
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (snapshot, session);
        Ok(())
    }
    async fn load_binding(&self, project: &str, session: &str) -> Result<Option<session_engine::agent_binding::AgentBinding>> { crate::session::agent_binding::load_scoped(project, session).await }
    async fn agent_catalog(&self) -> Result<HashMap<String, config_agent::AgentDefinitionConfig>> {
        let config = crate::config::get_current().await?;
        Ok(crate::agent::catalog::merged_agent_catalog(&config.agents))
    }
    async fn prepare_binding(&self, session: &str, agent: &str, model: Option<&str>, directory: &Path) -> Result<session_engine::agent_binding::AgentBinding> {
        Ok(crate::session::agent_binding::prepare(session, Some(agent), None, model, directory).await?.0)
    }
    fn locked_model(&self, session: &str, directory: &str) -> Result<Option<vw_api_acp_model::acp_model_source::AcpSessionModelSelection>> { crate::session::llm::locked_acp_model(session, directory).map_err(anyhow::Error::msg) }
    async fn persist_turn(&self, ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage, route: &ChannelRouteSelection, response: &str, processed: ChannelProcessorResponse) -> Result<Option<String>> { super::super::session_turn::persist_channel_turn(ctx, msg, route, response, processed).await }
    async fn take_reply(&self, snapshot: &ChannelMessageSnapshot, msg: &traits::ChannelMessage) -> Result<Option<(String, Option<String>)>> { super::super::session_turn::take_committed_channel_reply(snapshot, msg).await }
    async fn retry_reply(&self, snapshot: &ChannelMessageSnapshot, session: &str, msg: &traits::ChannelMessage, reply: String) -> Result<()> { super::super::session_turn::mark_channel_reply_retryable(snapshot, session, msg, reply).await }
}
