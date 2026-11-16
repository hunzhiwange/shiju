//! Native instances and scoped IO used by the session engine.
use super::types::{Request, ResolvedRequestModel, StreamEvent, ToolSessionState};
use crate::session::prompt;
use crate::{
    config::Config,
    tools::{ToolRuntimeContext, ToolUseContext},
};
use serde_json::Value;
use session_engine::session_memory::Session;
use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
};
pub(crate) struct AgentRunnerHost;
#[path = "runner_admission.rs"]
mod admission;
// RunnerPrompt is implemented beside its SystemPrompt owner.
impl session_engine::runner_host::RunnerHost for AgentRunnerHost {
    type CtxHost = crate::tools::context::AgentContextHost;
    type ImageExtractor = crate::multimodal::ocr::SessionImageTextExtractor;
    fn image_extractor() -> Self::ImageExtractor {
        Self::ImageExtractor::new()
    }
    type Admission = (
        plugin_hooks::hook_async::AsyncHookTurn,
        crate::plugins::session_snapshot::PluginTurnSnapshot,
    );
    type Prompt = crate::session::prompt::SystemPrompt;
    type Connections = Arc<crate::plugins::McpSessionConnections>;
    type Stream = super::llm_runner::AgentLlmStreamHost;
    type Prefetch = super::prefetch::AgentPrefetchHost;
    fn config() -> anyhow::Result<Config> {
        prompt::block_on(crate::config::get_current())
    }
    fn apply_binding(session: &str, config: &mut Config) -> anyhow::Result<()> {
        prompt::block_on(crate::session::agent_binding::apply_to_config(session, config))
    }
    fn native_defaults(
        config: &Config,
        key: &str,
        model: Option<&str>,
        workspace: &Path,
    ) -> anyhow::Result<Option<session_engine::capability_policy::NativeAgentDefaults>> {
        prompt::block_on(crate::session::capability_policy::native_agent_defaults(
            config, key, model, workspace,
        ))
    }
    fn providers() -> Result<std::collections::HashMap<String, model_types::Info>, String> {
        prompt::block_on(crate::provider::provider::list())
            .map_err(crate::session::llm::catalog_error_string)
    }
    fn acp_model_source(
        options: &Value,
    ) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String> {
        crate::session::llm::acp_model_source(options)
    }
    fn is_acp_model(model: &model_types::Model) -> bool {
        crate::session::llm::is_acp_model(model)
    }
    fn message_id() -> Result<String, String> {
        crate::id::ascending(crate::id::Prefix::Message, None).map_err(|error| error.to_string())
    }
    fn restore_context(session: &str, source: &[Value]) -> Result<Vec<Value>, String> {
        prompt::block_on(crate::session::compaction_checkpoint::restore(session, source))
    }
    fn admit_context(
        req: &Request,
        config: &Config,
        defaults: Option<session_engine::capability_policy::NativeAgentDefaults>,
        selected: &ResolvedRequestModel,
        auto: bool,
        workspace: &Path,
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> Option<(ToolRuntimeContext, Self::Admission)> {
        admission::admit_context(req, config, defaults, selected, auto, workspace, sink)
    }
    fn load_skills(
        workspace: &Path,
        config: &Config,
        sources: &crate::acp::mounts::SessionSkillSources,
        strict: bool,
    ) -> anyhow::Result<Vec<crate::skills::Skill>> {
        crate::skills::load_session_skills(workspace, config, sources, strict)
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn accessible_skills(
        skills: Vec<crate::skills::Skill>,
        context: Option<&ToolUseContext>,
    ) -> Vec<crate::skills::Skill> {
        crate::tools::skill::accessible_skills(skills, context)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_video_command(
        req: &Request,
        ctx: &ToolRuntimeContext,
        skills: &[crate::skills::Skill],
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> bool {
        crate::workflow::native::run_video_command(req, ctx, skills, sink)
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn run_bound_command(
        req: &Request,
        ctx: &ToolRuntimeContext,
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> bool {
        crate::workflow::native::run_bound_command(req, ctx, sink)
    }
    fn execution_tools(ctx: &ToolRuntimeContext) -> Vec<Box<dyn crate::tools::Tool>> {
        crate::tools::execution_tools_for_context(ctx)
    }
    fn tool_specs(ctx: &ToolRuntimeContext) -> Vec<crate::tools::ToolSpec> {
        crate::tools::tool_specs_for_context(ctx)
    }
    fn mcp_connections(ctx: &ToolRuntimeContext) -> Self::Connections {
        crate::tools::mcp_connections_for_context(ctx)
    }
    fn refresh_catalog(
        connections: &Self::Connections,
        catalog: &Arc<crate::tools::catalog::ToolCatalog>,
    ) -> anyhow::Result<()> {
        prompt::block_on(connections.refresh_tool_catalog(catalog))
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn request_catalog(
        context: &Arc<ToolUseContext>,
        snapshot: Arc<crate::tools::catalog::ToolCatalogSnapshot>,
    ) -> anyhow::Result<Arc<crate::tools::catalog::ToolCatalogSnapshot>> {
        crate::session::runtime::dynamic_hosts::request_catalog(context, snapshot)
    }
    fn new_discovery(
        specs: Vec<crate::tools::ToolSpec>,
    ) -> crate::tools::tool_search::ToolDiscoverySession {
        crate::tools::tool_search::ToolDiscoverySession::new(specs)
    }
    fn rebase_discovery(
        discovery: &crate::tools::tool_search::ToolDiscoverySession,
        specs: Vec<crate::tools::ToolSpec>,
    ) -> crate::tools::tool_search::ToolDiscoverySession {
        discovery.rebase(specs)
    }
    fn exposed_specs(
        discovery: &crate::tools::tool_search::ToolDiscoverySession,
    ) -> Vec<crate::tools::ToolSpec> {
        discovery.exposed_specs()
    }
    fn system_prompt(
        model: Option<&str>,
        root: Option<&str>,
        agent: Option<&str>,
        identity: &[String],
    ) -> Self::Prompt {
        prompt::system_with_agent_metadata(model, root, agent, identity)
    }
    fn runtime_prompt(
        prompt: Self::Prompt,
        model: Option<&str>,
        root: Option<&str>,
        ids: &[String],
        options: &Value,
    ) -> Self::Prompt {
        prompt.with_runtime_context_and_modes(
            model,
            root,
            ids,
            crate::session::system::RuntimePromptModes::from_options(options),
        )
    }
    fn deliver_model_events(
        context: &ToolUseContext,
        messages: &mut Vec<Value>,
    ) -> anyhow::Result<()> {
        crate::session::child_agent::coordinator::global_child_agent_coordinator()
            .deliver_model_events(context, messages)
    }
    fn team_instructions(session: &str, enabled: bool) -> String {
        prompt::block_on(crate::session::system::team_runtime_instructions(session, enabled))
    }

    fn parallel_limit(ctx: &ToolRuntimeContext, agent: &str) -> usize {
        crate::tools::agent_tool_parallel_limit_for_context(ctx, agent)
    }
    fn stop_hook(
        context: Arc<ToolUseContext>,
        interrupt: bool,
        active: bool,
        message: Option<&str>,
    ) -> anyhow::Result<plugin_hooks::hook_stop::Outcome> {
        prompt::block_on(crate::plugins::hook_stop::emit(context, interrupt, active, message))
    }
    fn read_todos(ctx: &ToolRuntimeContext) -> anyhow::Result<String> {
        Ok(crate::tools::todo::read(ctx)?)
    }

}

impl session_engine::runner_protocol::RunnerCheckpointHost for AgentRunnerHost {
    fn restore_skills(
        session: &str,
        agent: &str,
        history: &[Value],
    ) -> Result<Vec<crate::skills::InvokedSkillRecord>, String> {
        prompt::block_on(crate::session::skill_checkpoint::restore(session, agent, history))
    }
    fn save_skills(
        session: &str,
        agent: &str,
        history: &[Value],
        records: &[crate::skills::InvokedSkillRecord],
    ) -> Result<(), String> {
        prompt::block_on(crate::session::skill_checkpoint::save(session, agent, history, records))
    }
    fn save_context(
        session: &str,
        source: &[Value],
        summary: &str,
        retained_user: Option<Value>,
    ) -> Result<(), String> {
        prompt::block_on(crate::session::compaction_checkpoint::save(
            session,
            source,
            summary,
            retained_user,
        ))
    }
    fn compact_hook(
        session: &str,
        conversation: &str,
        model: &str,
        automatic: bool,
        completed: bool,
    ) -> Result<(), String> {
        prompt::block_on(crate::plugins::hook_compact::emit(
            session,
            conversation,
            model,
            automatic,
            completed,
        ))
        .map_err(|error| error.to_string())
    }
}
impl session_engine::compaction_summary::SummaryHost for AgentRunnerHost {
    fn run_llm_step(
        session_id: &str,
        messages: &[Value],
        system: &[String],
        model: Option<String>,
        options: &Value,
        available_tools: &[crate::tools::ToolSpec],
        allowed_tools: &HashSet<String>,
        on_event: &mut dyn FnMut(StreamEvent) -> bool,
        cancellation: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<super::llm_runner::LlmStep, String> {
        super::llm_runner::run_llm_step(
            session_id,
            messages,
            system,
            model,
            options,
            available_tools,
            allowed_tools,
            on_event,
            cancellation,
        )
    }
}

impl session_engine::helpers::HelperHost for AgentRunnerHost {
    fn specs(model: Option<&str>) -> Vec<crate::tools::ToolSpec> {
        crate::tools::registry::specs(model)
    }
    fn sanitize_tool_input(name: &str, arguments: &str) -> String {
        super::utils::sanitize_tool_input(name, arguments)
    }
}
impl session_engine::artifacts::ArtifactHost for AgentRunnerHost {
    fn build_prompt(session: &Session, model: Option<&str>, root: Option<&str>) -> String {
        super::prompting::build_prompt(session, model, root, None)
    }
    fn persist_ai_call_payload(session: &str, stream: u64, payload: &Value, scope: Option<&str>) {
        let _ = crate::session::ui_store::persist_ai_call_payload(session, stream, payload, scope);
    }
}
