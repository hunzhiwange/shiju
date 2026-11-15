pub(super) use desktop_session::message::chat::processing_timing::{start_tail_assistant_processing, complete_tail_assistant_processing, ProcessingElapsed};
pub(super) fn pause_runtime_processing_timers(runtime: &mut crate::app::state::SessionRuntimeState) -> ProcessingElapsed {
 desktop_session::message::chat::processing_timing::pause_runtime_processing_timers(&mut runtime.processing_timer, &mut runtime.llm_timer)
}
