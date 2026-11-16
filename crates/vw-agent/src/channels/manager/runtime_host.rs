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
    // 命令业务由运行库直接执行；这里只保留会话原始操作。
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


#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl channel_runtime::command_host::CommandHost for RuntimeHost {
    fn command_tenant(&self) -> vw_api_transport::id::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn prepare_catalog(&self) { crate::provider::provider::ensure_host(); }
    fn command_now_ms(&self) -> u64 { crate::session::session::now_ms() }
    fn session_activity(&self, id: &str) -> session_port::SessionStatus { crate::session::status::get(id) }
    async fn in_project_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) {
        let context = crate::project::instance::Context { directory: snapshot.directory.to_string_lossy().into_owned(), worktree: snapshot.project.worktree.clone(), project: snapshot.project.clone() };
        crate::project::instance::provide_context(context, operation).await;
    }
    async fn in_session_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) {
        <Self as ChannelHost>::in_scope(self, snapshot, operation).await;
    }
    fn get_session<'a>(&'a self, id: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<session_info::Info, session_info::Error>> + Send + 'a>> { Box::pin(crate::session::session::get_any(id)) }
    async fn get_scoped_session(&self, id: &str) -> Result<session_info::Info, session_info::Error> { crate::session::session::get(id).await }
    #[cfg(not(target_arch = "wasm32"))]
    async fn reserve_session(&self, snapshot: &ChannelMessageSnapshot) -> Result<session_info::Info> {
        let directory = snapshot.directory.to_string_lossy().to_string();
        let context = crate::project::instance::Context { directory: directory.clone(), worktree: snapshot.project.worktree.clone(), project: snapshot.project.clone() };
        Ok(crate::project::instance::provide_context(context, crate::session::session::reserve_next(crate::session::session::CreateInput { parent_id: None, title: None, directory, permission: None })).await?)
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn discard_session(&self, snapshot: &ChannelMessageSnapshot, info: session_info::Info) -> Result<()> {
        let tenant = snapshot.scope.chat.tenant.clone();
        let project = info.project_id;
        let id = info.id;
        crate::global_database::run_isolated(move || crate::session::ui_store::discard_pending_session(&tenant, &project, &id)).await??;
        Ok(())
    }
    async fn set_session_title(&self, id: &str, title: &str) -> Result<()> {
        crate::session::session::update_any(id, |info| info.title = title.into()).await?; Ok(())
    }
    async fn set_session_archived(&self, id: &str, archived: Option<u64>) -> Result<()> {
        crate::session::session::update_any(id, |info| info.time.archived = archived).await?; Ok(())
    }
    async fn fork_session(&self, id: &str, anchor: Option<&str>) -> Result<session_info::Info> { Ok(crate::session::session::fork(id, anchor).await?) }
    async fn remove_session(&self, id: &str) -> Result<()> { Ok(crate::session::session::remove_any(id).await?) }
    async fn revert_session(&self, id: &str, message: &str, part: Option<&str>) -> Result<session_info::Info> {
        Ok(crate::session::revert::revert(crate::session::revert::RevertInput { session_id: id.into(), message_id: message.into(), part_id: part.map(Into::into) }).await?)
    }
    async fn cleanup_revert(&self, info: &session_info::Info) -> Result<()> { Ok(crate::session::revert::cleanup(info).await?) }
    async fn compact_session(&self, id: &str, anchor: &str, auto: bool) -> Result<session_engine::compaction::CompactionOutcome> { Ok(Box::pin(crate::session::compaction::process_session(id, anchor, auto)).await?) }
    async fn load_ui_session(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<session_port::ui_types::ChatSession>> {
        let tenant = snapshot.scope.chat.tenant.clone(); let project = snapshot.scope.project.clone(); let id = id.to_owned();
        #[cfg(not(target_arch = "wasm32"))]
        { crate::global_database::run_isolated(move || crate::session::ui_store::load_session_scoped_for_tenant_fallible(&tenant, &id, Some(&project))).await? }
        #[cfg(target_arch = "wasm32")]
        { Ok(crate::session::ui_store::load_session_scoped_for_tenant(&tenant, &id, Some(&project))) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn load_session_preview(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<String>> {
        let tenant = snapshot.scope.chat.tenant.clone(); let project = snapshot.scope.project.clone(); let id = id.to_owned();
        crate::global_database::run_isolated(move || Ok(crate::session::ui_store::load_session_meta_scoped_for_tenant(&tenant, &id, Some(&project))?.and_then(|meta| meta.last_content))).await?
    }
    fn create_task(&self, project: &str, priority: u32, model: Option<&str>, prompt: &str) -> std::io::Result<String> {
        let mut task = crate::app::task::Task::new(priority);
        task.prompt = prompt.into(); if let Some(model) = model { task.model = model.into(); }
        crate::app::task::create_task(project, task).map(|task| task.id)
    }
}

impl channel_runtime::startup_host::ChannelStartupHost for RuntimeHost {
    fn tenant(&self) -> vw_api_transport::id::TenantScope {
        crate::session::ui_store::current_tenant_scope()
    }
    fn tools(&self, config: &Config, security: &Arc<crate::security::SecurityPolicy>, runtime: Arc<dyn runtime_types::RuntimeAdapter>, memory: Arc<dyn crate::memory::Memory>) -> Vec<Box<dyn crate::tools::Tool>> {
        crate::tools::all_tools_with_runtime(
            Arc::new(config.clone()), security, runtime, memory, &config.browser,
            &config.http_request, &config.web_fetch, &config.workspace_dir, &config.agents,
            config.api_key.as_deref(), config, None,
        )
    }
    fn skills(&self, workspace: &Path, config: &Config) -> Vec<agent_skills::Skill> {
        crate::skills::load_skills_with_config(workspace, config)
    }
    fn hooks(&self, config: &Config) -> Result<Option<Arc<crate::hooks::HookRunner>>> {
        Ok(crate::hooks::build_hook_runtime(config)?.runner_snapshot())
    }
}
