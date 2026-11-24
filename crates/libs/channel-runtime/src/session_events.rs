//! 将共享处理器事件投影为通道事件。
pub fn channel_prompt_event(event: session_engine::processor_events::StreamEvent) -> Option<session_port::PromptEvent> {
    use session_engine::processor_events::StreamEvent as E;
    use session_port::{PromptEvent as P, PromptTermination as T};
    Some(match event {
        E::ModelToolCalls(_) => return None,
        E::LlmRawArtifact(value) => P::LlmRawArtifact(value),
        E::Delta(value) => P::Text(value),
        E::Tool(value) => P::Tool(value),
        E::Done(value) => P::Terminated(T::Completed(value)),
        E::Error(value) => P::Terminated(T::Failed(value)),
        E::StepStart { step_index, created_ms, model } => P::StepStart { step_index, created_ms, model },
        E::StepFinish { step_index, finished_ms, usage, finish_reason, model } => P::StepFinish { step_index, finished_ms, usage, finish_reason, model },
        E::PostToolRound { step_index } => P::PostToolRound { step_index },
        E::ModelRecovery(value) => P::ModelRecovery(value),
        E::TodoContinuation => P::TodoContinuation,
        E::Compaction { phase } => P::Compaction { phase },
        E::ContextLoaded { paths } => P::ContextLoaded { paths },
    })
}
