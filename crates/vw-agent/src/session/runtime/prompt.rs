//! Shared prompt execution with the concrete tool context binding.
pub(crate) type EventSink = session_engine::runtime::prompt::EventSink;
pub(crate) fn start_prompt_with_runner<R>(request: crate::session::processor::Request, runner: R) -> session_port::PromptTurn
where R: FnOnce(crate::session::processor::Request, EventSink) + Send + 'static {
    session_engine::runtime::prompt::start_prompt_with_runner::<crate::tools::context::AgentContextHost, R>(request, runner)
}
