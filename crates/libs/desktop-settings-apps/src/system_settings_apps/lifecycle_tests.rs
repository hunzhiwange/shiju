use vw_api_app_operations::{AppActionPreview, AppActionPreviewId, AppActionPreviewRequest, AppActionSubmitRequest, AppActionTarget, AppLifecycleAction, AppOperationPhase, AppOperationStatus};
use super::lifecycle::LifecycleState;
use {
    vw_api_transport::id::TaskId,
    vw_api_task::TaskStatus,
};
fn status(id: &str, phase: AppOperationPhase, status: TaskStatus) -> AppOperationStatus {
    AppOperationStatus { operation_id: TaskId(id.into()), status, phase }
}
#[test]
fn cc_apps_desktop_lifecycle_state_rejects_other_operation_and_terminal_regression() {
    let mut state = LifecycleState::default();
    assert!(state.accept_status(status("same", AppOperationPhase::Executing, TaskStatus::Running)));
    assert!(!state.accept_status(status(
        "other",
        AppOperationPhase::Terminal,
        TaskStatus::Completed
    )));
    assert!(state.active());
    assert!(state.accept_status(status(
        "same",
        AppOperationPhase::Terminal,
        TaskStatus::Cancelled
    )));
    assert!(!state.accept_status(status(
        "same",
        AppOperationPhase::Executing,
        TaskStatus::Running
    )));
    assert_eq!(state.operation.unwrap().status, TaskStatus::Cancelled);
}
