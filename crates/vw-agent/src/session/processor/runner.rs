//! Project scope composition and thin compatibility bindings for session-engine.
use super::runner_host::AgentRunnerHost;
#[cfg(test)]
use super::types::CompactionPhase;
use super::types::{Request, StreamEvent};
#[cfg(test)]
use super::types::{ResolvedRequestModel, ToolSessionState};
#[cfg(test)]
use super::{llm_runner, tools_exec};
use crate::session::prompt;
#[cfg(test)]
use crate::session::session::{Role, Session};
#[cfg(test)]
use crate::session::ui_types as models;
#[cfg(test)]
use crate::tools::ToolRuntimeContext;
use crate::tools::ToolUseContext;
use serde_json::Value;
#[cfg(test)]
use session_engine::helpers::is_acp_request;
#[cfg(test)]
use session_engine::runner::{
    EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT, EMPTY_RESPONSE_RETRY_PROMPT,
};
pub(crate) use session_engine::runner_protocol::provider_request_options;
#[cfg(test)]
use session_engine::runner_protocol::{
    delegated_skill_policy_resolution, prepare_context_before_llm_step,
};
#[cfg(test)]
use std::collections::{BTreeSet, HashSet};
#[cfg(test)]
#[derive(Default)]
struct DesktopSessionProtocol(session_engine::runner_protocol::DesktopSessionProtocol);
#[cfg(test)]
impl std::ops::Deref for DesktopSessionProtocol {
    type Target = session_engine::runner_protocol::DesktopSessionProtocol;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[cfg(test)]
impl std::ops::DerefMut for DesktopSessionProtocol {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[cfg(test)]
impl DesktopSessionProtocol {
    fn new(
        skills: Vec<crate::skills::Skill>,
        agent_id: impl Into<String>,
        context_window_tokens: usize,
    ) -> Self {
        Self(session_engine::runner_protocol::DesktopSessionProtocol::new(
            skills,
            agent_id,
            context_window_tokens,
        ))
    }
    fn prepare_native_skill_context(
        &mut self,
        req: &Request,
        history: &[Value],
        skill_policy: Option<&crate::session::capability_policy::CapabilityPolicyResolution>,
        agents_referenced_skills: &BTreeSet<String>,
        user_referenced_skills: &BTreeSet<String>,
    ) -> Result<(), String> {
        self.0.prepare_native_skill_context::<AgentRunnerHost>(
            &req.session,
            &req.options,
            history,
            skill_policy,
            agents_referenced_skills,
            user_referenced_skills,
        )
    }
}

pub fn run(req: Request, on_event: impl FnMut(StreamEvent) -> bool + Send + 'static) {
    run_with_persisted_history(req, None, on_event);
}

pub(crate) fn run_with_persisted_history(
    req: Request,
    persisted_history: Option<Vec<Value>>,
    mut on_event: impl FnMut(StreamEvent) -> bool + Send + 'static,
) {
    let Some(root) = req.root.as_deref().filter(|root| !root.trim().is_empty()) else {
        return run_in_project(req, persisted_history, on_event);
    };
    #[cfg(not(target_arch = "wasm32"))]
    let _worktree_use = match crate::worktree::registry::use_lease(std::path::Path::new(root)) {
        Ok(lease) => lease,
        Err(error) => {
            on_event(StreamEvent::Error(error.to_string()));
            return;
        }
    };
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project =
        prompt::block_on(crate::project::from_directory(&tenant, std::path::Path::new(root)));
    match project {
        Ok((project, worktree)) => {
            let context = crate::project::instance::Context {
                directory: root.to_string(),
                worktree,
                project,
            };
            crate::project::instance::provide_context_sync(context, || {
                run_in_project(req, persisted_history, on_event)
            })
        }
        Err(error) => {
            on_event(StreamEvent::Error(format!("Failed to load request project: {error}")));
        }
    }
}

fn run_in_project(
    req: Request,
    persisted_history: Option<Vec<Value>>,
    on_event: impl FnMut(StreamEvent) -> bool + Send + 'static,
) {
    session_engine::runner::run::<AgentRunnerHost>(req, persisted_history, on_event)
}

#[cfg(test)]
fn request_provider_retry_policy(req: &Request) -> (u32, u64) {
    session_engine::runner::request_provider_retry_policy::<AgentRunnerHost>(req)
}
fn request_full_access_enabled(options: &Value) -> bool {
    session_engine::runner::request_full_access_enabled(options)
}
fn with_request_model_context(
    context: ToolUseContext,
    request_model: Option<&str>,
) -> ToolUseContext {
    session_engine::runner::with_request_model_context::<AgentRunnerHost>(context, request_model)
}
#[cfg(test)]
fn resolve_request_model(
    req: &Request,
    config: &crate::config::Config,
) -> Result<ResolvedRequestModel, String> {
    session_engine::runner::resolve_request_model::<AgentRunnerHost>(req, config)
}
#[cfg(test)]
fn enabled_identity_files_for_agent(
    config: &crate::config::Config,
    agent_key: Option<&str>,
) -> Vec<String> {
    session_engine::runner::enabled_identity_files_for_agent(config, agent_key)
}
#[cfg(test)]
fn should_handle_docs_request_locally(query: &str) -> bool {
    session_engine::runner::should_handle_docs_request_locally(query)
}
#[cfg(test)]
fn handle_docs_request(
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) {
    session_engine::runner::handle_docs_request::<AgentRunnerHost>(session, ctx, on_event)
}
#[cfg(test)]
fn handle_step_error(
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    empty_response_retries: &mut usize,
    step: &mut usize,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    message: String,
) -> bool {
    session_engine::runner::handle_step_error(
        session,
        llm_messages,
        empty_response_retries,
        step,
        on_event,
        message,
    )
}
#[cfg(test)]
fn handle_structured_tool_calls(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    base_system: &str,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    session_protocol: &mut DesktopSessionProtocol,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    parallel_tools: bool,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    session_engine::runner::handle_structured_tool_calls::<AgentRunnerHost>(
        req,
        session,
        ctx,
        allowed_tools,
        base_system,
        llm_messages,
        total_usage,
        on_event,
        tool_state,
        &mut session_protocol.0,
        step_out,
        step_text_preview,
        parallel_tools,
        is_last_step,
        step_index,
    )
}
#[cfg(test)]
fn handle_assistant_text_branch(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    empty_response_retries: &mut usize,
    step: &mut usize,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    empty_tools: &HashSet<String>,
    is_last_step: bool,
    step_index: u32,
    app_session_scope: Option<&str>,
    stop_hook_active: &mut bool,
) -> bool {
    session_engine::runner::handle_assistant_text_branch::<AgentRunnerHost>(
        req,
        session,
        ctx,
        allowed_tools,
        llm_messages,
        total_usage,
        on_event,
        tool_state,
        empty_response_retries,
        step,
        step_out,
        step_text_preview,
        empty_tools,
        is_last_step,
        step_index,
        app_session_scope,
        stop_hook_active,
    )
}
#[cfg(test)]
fn handle_empty_assistant_text(
    req: &Request,
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    empty_response_retries: &mut usize,
    step: &mut usize,
    reasoning_content: &str,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    session_engine::runner::handle_empty_assistant_text::<AgentRunnerHost>(
        req,
        session,
        llm_messages,
        on_event,
        empty_response_retries,
        step,
        reasoning_content,
        is_last_step,
        step_index,
    )
}
#[cfg(test)]
fn build_image_turn_context_for_request(
    req: &Request,
    session: &Session,
    config: &crate::config::Config,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) -> Result<Option<crate::app::agent::multimodal::context::ImageTurnContext>, String> {
    session_engine::runner::build_image_turn_context_for_request::<AgentRunnerHost>(
        req, session, config, on_event,
    )
}
#[cfg(test)]
fn compact_transient_history_for_request(
    session_id: &str,
    checkpoint_source: &[Value],
    llm_messages: &mut Vec<Value>,
    model: &str,
    reliability: &crate::config::ReliabilityConfig,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
    model_limit: &crate::provider::provider::ModelLimit,
) -> Result<bool, String> {
    session_engine::runner_protocol::compact_transient_history_for_request::<AgentRunnerHost>(
        session_id,
        checkpoint_source,
        llm_messages,
        model,
        reliability,
        on_event,
        cancellation,
        model_limit,
    )
}
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/runner_tests.rs"]
mod runner_tests;
