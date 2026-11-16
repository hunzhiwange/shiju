//! Application system prompt composition adapter.
use super::Session;
pub(crate) fn build_prompt(session: &Session, model: Option<&str>, root: Option<&str>, extra_assistant: Option<&str>) -> String {
    session_engine::prompting::build_prompt(session, &crate::session::prompt::system(model, root), extra_assistant)
}
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/prompting_tests.rs"]
mod prompting_tests;
