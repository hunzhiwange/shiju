//! Chat routing and stream bridging use the production domain entry points.
use crate::agent_stream_events::StreamMessage;
use crate::message::chat::events::ChatMessage;
use crate::message::chat::routing::Route;
use crate::session::agent::SessionRuntimeState;
type Event = ChatMessage<(), (), (), (), (), ()>;

#[test]
fn chat_routes_preserve_each_domain_boundary() {
    for (event, route) in [
        (Event::AcpOptionsClose, Route::AcpOptions),
        (Event::InputChanged("draft".into()), Route::Input),
        (Event::TrajectoryDetailsClosed, Route::Trajectory),
        (Event::AgentStreamDelta(12, "delta".into()), Route::Stream),
        (Event::AppendText("context".into()), Route::Context),
        (Event::SendPressed, Route::Session),
    ] {
        assert_eq!(event.route(), route);
    }
}

#[test]
fn stream_bridge_preserves_request_session_and_payload() {
    let event: Event = StreamMessage::AgentStreamDelta(42, "delta".into()).into();
    assert!(matches!(event, Event::AgentStreamDelta(42, ref value) if value == "delta"));
    let event: Event =
        StreamMessage::PlanRunSessionDelta { session_id: "child".into(), delta: "text".into() }
            .into();
    assert!(
        matches!(event, Event::PlanRunSessionDelta { session_id, delta } if session_id == "child" && delta == "text")
    );
}

#[test]
fn acp_refresh_isolates_unstarted_fork_and_serializes_requests() {
    let mut runtime = SessionRuntimeState::<()>::default();
    assert!(runtime.begin_acp_source_refresh("session").is_none());
    runtime.acp_agent = Some("agent".into());
    runtime.fork_agent_selected = true;
    runtime.fork_execution_started = Some(false);
    let (agent, request, draft) = runtime.begin_acp_source_refresh("session").unwrap();
    assert_eq!(agent, "agent");
    assert!(draft.is_some());
    assert_eq!(runtime.acp_options_state.source_request_id, Some(request));
    assert!(runtime.acp_options_state.pending);
    assert!(runtime.begin_acp_source_refresh("session").is_none());
}

#[test]
fn trajectory_raw_refresh_normalizes_blank_turn_id() {
    use crate::message::chat::trajectory::RawLoadState;
    use std::collections::{HashMap, HashSet};
    let steps = vec![session_ui_types::ChatSessionStep {
        index: 12,
        turn_id: Some("  \t".into()),
        started_ms: 10,
        finished_ms: Some(20),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: Default::default(),
        cost_usd: None,
        finish_reason: Some("stop".into()),
        model: Some("test-model".into()),
        tool_timings: Vec::new(),
    }];
    let indices = HashSet::new();
    let raw = HashMap::new();
    let mut loading = HashSet::new();
    let mut attempts = HashMap::new();
    let mut errors = HashMap::new();
    let mut state = RawLoadState {
        steps: &steps,
        indices: &indices,
        raw: &raw,
        loading: &mut loading,
        attempts: &mut attempts,
        errors: &mut errors,
    };
    let missing = state.missing("active-session", None);
    let legacy_key = ("active-session".to_string(), "legacy-step-12".to_string(), 12);
    assert!(!missing.is_empty());
    assert!(state.loading.contains(&legacy_key));
    assert_eq!(state.attempts.get(&legacy_key), Some(&1));
}
