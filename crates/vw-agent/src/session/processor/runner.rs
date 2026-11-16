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
use session_engine::runner_protocol::DesktopSessionProtocol;

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
#[path = "../../../../libs/session-engine/tests/host_suites/runner_tests.rs"]
mod runner_tests;
