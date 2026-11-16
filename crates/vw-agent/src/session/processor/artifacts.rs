//! Host persistence and prompt adapter for processor artifacts.
use super::types::Request;
use super::llm_runner::LlmStep;
use crate::session::session::Session;
use crate::session::ui_types as models;
use super::runner_host::AgentRunnerHost as AgentArtifactHost;
fn artifact_request(req: &Request) -> session_engine::artifacts::ArtifactRequest<'_> {
    session_engine::artifacts::ArtifactRequest { stream: req.stream, session: &req.session, root: req.root.as_deref(), model: req.model.as_deref(), options: &req.options, persist_app_session_artifacts: req.persist_app_session_artifacts }
}
pub(crate) fn persist_step_ai_call_payload(
    req: &Request,
    session: &Session,
    total_usage: &models::TokenUsage,
    step: &LlmStep,
    step_index: u32,
    app_session_scope: Option<&str>,
) { session_engine::artifacts::persist_step_ai_call_payload::<AgentArtifactHost>(&artifact_request(req), session, total_usage, step, step_index, app_session_scope) }
pub(crate) fn persist_final_ai_call_payload(
    req: &Request,
    session: &Session,
    total_usage: &models::TokenUsage,
    answer: &str,
    app_session_scope: Option<&str>,
    should_persist: bool,
) { session_engine::artifacts::persist_final_ai_call_payload::<AgentArtifactHost>(&artifact_request(req), session, total_usage, answer, app_session_scope, should_persist) }
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/artifacts_tests.rs"]
mod artifacts_tests;
