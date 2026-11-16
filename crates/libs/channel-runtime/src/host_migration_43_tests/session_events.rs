use crate::session_events::channel_prompt_event;
use session_engine::processor_events::StreamEvent as E;
use session_port::{PromptEvent as P, PromptTermination as T};

#[test]
fn channel_prompt_event_preserves_text_and_terminal_outcomes() {
    assert!(matches!(channel_prompt_event(E::Delta("delta".into())), Some(P::Text(text)) if text == "delta"));
    assert!(matches!(channel_prompt_event(E::Error("failure".into())), Some(P::Terminated(T::Failed(text))) if text == "failure"));
    assert!(matches!(channel_prompt_event(E::Done(Default::default())), Some(P::Terminated(T::Completed(_)))));
    assert!(channel_prompt_event(E::ModelToolCalls(Vec::new())).is_none());
}

#[test]
fn channel_prompt_event_preserves_step_and_context_metadata() {
    assert!(matches!(channel_prompt_event(E::StepStart { step_index: 2, created_ms: 41, model: Some("model".into()) }),
        Some(P::StepStart { step_index: 2, created_ms: 41, model: Some(model) }) if model == "model"));
    assert!(matches!(channel_prompt_event(E::StepFinish { step_index: 2, finished_ms: 42, usage: Default::default(), finish_reason: Some("stop".into()), model: None }),
        Some(P::StepFinish { step_index: 2, finished_ms: 42, finish_reason: Some(reason), .. }) if reason == "stop"));
    assert!(matches!(channel_prompt_event(E::ContextLoaded { paths: vec!["AGENTS.md".into()] }),
        Some(P::ContextLoaded { paths }) if paths == vec!["AGENTS.md"]));
    assert!(matches!(channel_prompt_event(E::PostToolRound { step_index: 2 }), Some(P::PostToolRound { step_index: 2 })));
    assert!(matches!(channel_prompt_event(E::TodoContinuation), Some(P::TodoContinuation)));
}
