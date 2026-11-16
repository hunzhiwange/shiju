//! Concrete IO ports used by the runner. No callback owns a turn or model round.
use crate::{
    processor_events::StreamEvent,
    runner::Request,
    runner_types::{ResolvedRequestModel, ToolSessionState},
    session_memory::Session,
};
use agent_config::runtime_schema::Config;
use agent_skills::Skill;
use serde_json::Value;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
    sync::Arc,
};
use tool_assembly::runtime::{RuntimeHost, SessionToolset, ToolRuntimeContext};
use tool_catalog::catalog::{ToolCatalog, ToolCatalogSnapshot};
use tool_context::context::ToolUseContext;

pub trait RunnerPrompt {
    fn content(&self) -> &str;
    fn visible_instruction_paths(&self) -> &[String];
    fn instruction_context(&self) -> &[session_instructions::Resolved];
    fn runtime_parts(&self, primary: Option<&str>, last: bool) -> Vec<String>;
}

pub trait RunnerHost:
    Sized
    + crate::runner_protocol::RunnerCheckpointHost
    + crate::compaction_summary::SummaryHost
    + crate::helpers::HelperHost
    + crate::artifacts::ArtifactHost
{
    type CtxHost: RuntimeHost<
            InvokedSkillRecord = agent_skills::invoked::InvokedSkillRecord,
            ToolCatalog = ToolCatalog,
            ToolCatalogSnapshot = ToolCatalogSnapshot,
            ImageAssetScope = agent_multimodal::assets::ImageAssetScope,
        >;
    type Admission;
    type ImageExtractor: agent_multimodal::ocr::ImageTextExtractor;
    fn image_extractor() -> Self::ImageExtractor;
    type Prompt: RunnerPrompt;
    type Connections;
    type PlannedCall;
    type Stream: crate::llm_runner::LlmStreamHost;
    type Prefetch: crate::prefetch::PrefetchHost<
            Context = ToolRuntimeContext<Self::CtxHost>,
            State = ToolSessionState,
        >;
    fn config() -> anyhow::Result<Config>;
    fn apply_binding(session: &str, config: &mut Config) -> anyhow::Result<()>;
    fn native_defaults(
        config: &Config,
        key: &str,
        model: Option<&str>,
        workspace: &Path,
    ) -> anyhow::Result<Option<crate::capability_policy::NativeAgentDefaults>>;
    fn providers() -> Result<HashMap<String, model_types::Info>, String>;
    fn acp_model_source(
        options: &Value,
    ) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String>;
    fn is_acp_model(model: &model_types::Model) -> bool;
    fn message_id() -> Result<String, String>;
    fn restore_context(session: &str, source: &[Value]) -> Result<Vec<Value>, String>;
    fn admit_context(
        req: &Request<Self>,
        config: &Config,
        defaults: Option<crate::capability_policy::NativeAgentDefaults>,
        selected: &ResolvedRequestModel,
        auto: bool,
        workspace: &Path,
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> Option<(ToolRuntimeContext<Self::CtxHost>, Self::Admission)>;
    fn load_skills(
        workspace: &Path,
        config: &Config,
        sources: &<Self::CtxHost as tool_context::host::ContextHost>::SessionSkillSources,
        strict: bool,
    ) -> anyhow::Result<Vec<Skill>>;
    #[cfg(not(target_arch = "wasm32"))]
    fn accessible_skills(
        skills: Vec<Skill>,
        context: Option<&ToolUseContext<Self::CtxHost>>,
    ) -> Vec<Skill>;
    fn referenced_skills<'a>(
        skills: &[Skill],
        texts: impl IntoIterator<Item = &'a str>,
    ) -> BTreeSet<String>;
    fn explicit_skill_request(query: &str) -> bool;
    fn invocable_skills(skills: &[Skill]) -> BTreeSet<String>;
    #[cfg(not(target_arch = "wasm32"))]
    fn run_video_command(
        req: &Request<Self>,
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        skills: &[Skill],
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> bool;
    #[cfg(not(target_arch = "wasm32"))]
    fn run_bound_command(
        req: &Request<Self>,
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        sink: &mut impl FnMut(StreamEvent) -> bool,
    ) -> bool;
    fn execution_tools(ctx: &ToolRuntimeContext<Self::CtxHost>) -> Vec<Box<dyn tool_types::Tool>>;
    fn tool_specs(ctx: &ToolRuntimeContext<Self::CtxHost>) -> Vec<tool_types::ToolSpec>;
    fn mcp_connections(ctx: &ToolRuntimeContext<Self::CtxHost>) -> Self::Connections;
    fn refresh_catalog(
        connections: &Self::Connections,
        catalog: &Arc<ToolCatalog>,
    ) -> anyhow::Result<()>;
    #[cfg(not(target_arch = "wasm32"))]
    fn request_catalog(
        context: &Arc<ToolUseContext<Self::CtxHost>>,
        snapshot: Arc<ToolCatalogSnapshot>,
    ) -> anyhow::Result<Arc<ToolCatalogSnapshot>>;
    fn new_discovery(
        specs: Vec<tool_types::ToolSpec>,
    ) -> <Self::CtxHost as tool_context::host::ContextHost>::ToolDiscoverySession;
    fn rebase_discovery(
        discovery: &<Self::CtxHost as tool_context::host::ContextHost>::ToolDiscoverySession,
        specs: Vec<tool_types::ToolSpec>,
    ) -> <Self::CtxHost as tool_context::host::ContextHost>::ToolDiscoverySession;
    fn exposed_specs(
        discovery: &<Self::CtxHost as tool_context::host::ContextHost>::ToolDiscoverySession,
    ) -> Vec<tool_types::ToolSpec>;
    fn system_prompt(
        model: Option<&str>,
        root: Option<&str>,
        agent: Option<&str>,
        identity: &[String],
    ) -> Self::Prompt;
    fn runtime_prompt(
        prompt: Self::Prompt,
        model: Option<&str>,
        root: Option<&str>,
        ids: &[String],
        options: &Value,
    ) -> Self::Prompt;
    fn deliver_model_events(
        context: &ToolUseContext<Self::CtxHost>,
        messages: &mut Vec<Value>,
    ) -> anyhow::Result<()>;
    fn team_instructions(session: &str, enabled: bool) -> String;
    fn planned_call(
        name: &str,
        input: &str,
        ctx: ToolRuntimeContext<Self::CtxHost>,
    ) -> Self::PlannedCall;
    fn parallel_limit(ctx: &ToolRuntimeContext<Self::CtxHost>, agent: &str) -> usize;
    fn stop_hook(
        context: Arc<ToolUseContext<Self::CtxHost>>,
        interrupt: bool,
        active: bool,
        message: Option<&str>,
    ) -> anyhow::Result<plugin_hooks::hook_stop::Outcome>;
    fn read_todos(ctx: &ToolRuntimeContext<Self::CtxHost>) -> anyhow::Result<String>;
    fn with_request_tool_catalog(
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        session_tools: &SessionToolset,
        snapshot: &Arc<ToolCatalogSnapshot>,
    ) -> ToolRuntimeContext<Self::CtxHost>;
    fn is_agent_tool(tool: &str) -> bool;

    fn run_agent_tools_and_record(
        session: &mut Session,
        planned: &[Self::PlannedCall],
        max_parallel: usize,
        emit_ui: bool,
        on_event: &mut impl FnMut(StreamEvent) -> bool,
        tool_state: &mut ToolSessionState,
    ) -> Vec<String>;
    fn run_tool_and_record_with_protocol(
        session: &mut Session,
        name: &str,
        input: &str,
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        emit_ui: bool,
        on_event: &mut impl FnMut(StreamEvent) -> bool,
        tool_state: &mut ToolSessionState,
    ) -> Option<crate::runner_types::ToolProtocolOutput>;
    fn run_tool_and_record(
        session: &mut Session,
        name: &str,
        input: &str,
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        emit_ui: bool,
        on_event: &mut impl FnMut(StreamEvent) -> bool,
        tool_state: &mut ToolSessionState,
    ) -> Option<String>;
    #[cfg(not(target_arch = "wasm32"))]
    fn run_native_search_and_record(
        session: &mut Session,
        call: &model_types::ToolCall,
        ctx: &ToolRuntimeContext<Self::CtxHost>,
        on_event: &mut impl FnMut(StreamEvent) -> bool,
        tool_state: &mut ToolSessionState,
    ) -> anyhow::Result<crate::native_history::NativeHistoryItem>;
    #[cfg(not(target_arch = "wasm32"))]
    fn record_native_history_pair(
        session: &mut Session,
        call: crate::native_history::NativeHistoryItem,
        output: crate::native_history::NativeHistoryItem,
        on_event: &mut impl FnMut(StreamEvent) -> bool,
    );
    fn list_docs(root: Option<&String>) -> Result<Vec<String>, String>;
}
