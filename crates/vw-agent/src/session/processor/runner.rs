//! Project scope composition and thin compatibility bindings for session-engine.
use super::runner_host::AgentRunnerHost;
#[cfg(test)]
use super::types::CompactionPhase;
use super::types::{Request, StreamEvent};
#[cfg(test)]
use super::types::ToolSessionState;
#[cfg(test)]
use super::{llm_runner, tools_exec};
use crate::session::prompt;
#[cfg(test)]
use crate::session::session::{Role, Session};
#[cfg(test)]
use crate::session::ui_types as models;
#[cfg(test)]
use crate::tools::ToolRuntimeContext;
#[cfg(test)]
use crate::tools::ToolUseContext;
use serde_json::Value;
#[cfg(test)]
use session_engine::helpers::is_acp_request;
#[cfg(test)]
use session_engine::runner::{
    EMPTY_RESPONSE_RETRY_LIMIT, EMPTY_RESPONSE_RETRY_PROMPT,
};
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
#[path = "../../../../libs/session-engine/tests/host_suites/runner_tests.rs"]
mod runner_tests;
