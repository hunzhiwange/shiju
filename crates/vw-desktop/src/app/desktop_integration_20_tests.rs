use super::LifecycleState;
use vw_gateway_client::vw_api_types::{agent_apps::*, id::TaskId, task::TaskStatus};

#[test]
fn desktop_integration_20_preserves_expected_behavior() {
    let mut state = LifecycleState::default();
    let snapshot = |id: &str, phase, status| AppOperationStatus {
        operation_id: TaskId(id.into()), phase, status,
    };
    assert!(state.accept_status(snapshot("current", AppOperationPhase::Executing, TaskStatus::Running)));
    state.cancel_requested = true;
    // Requesting cancellation must not invent a terminal status.
    assert!(state.active());
    assert!(state.accept_status(snapshot("current", AppOperationPhase::Postflight, TaskStatus::Running)));
    assert!(state.cancel_requested);
    assert!(!state.accept_status(snapshot("other", AppOperationPhase::Terminal, TaskStatus::Cancelled)));
    assert!(state.cancel_requested);
    // A task may finish before cooperative cancellation takes effect.
    assert!(state.accept_status(snapshot("current", AppOperationPhase::Terminal, TaskStatus::Completed)));
    assert!(!state.cancel_requested);
    assert!(!state.active());
    assert_eq!(state.operation.as_ref().unwrap().status, TaskStatus::Completed);
    state.invalidate();
    assert!(!state.cancel_requested);
}
