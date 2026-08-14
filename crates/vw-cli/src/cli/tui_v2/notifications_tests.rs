use super::model::notification::{NotificationPriority, NotificationStore, TuiNotification};
use super::notifications::{
    ExitDecision, ExitFlow, builtin_status_line, osc_title, progress_bar, startup_notifications,
};
use super::state::TuiState;

fn notification(id: &str, priority: NotificationPriority, created_ms: u64) -> TuiNotification {
    TuiNotification {
        id: id.to_string(),
        message: format!("{id} token=must-hide"),
        priority,
        created_ms,
        expires_ms: Some(created_ms + 100),
        persistent: false,
    }
}

#[test]
fn tui_notifications_status_and_exit_flow_match_reference_priorities() {
    let mut store = NotificationStore::default();
    store.push(notification("info", NotificationPriority::Info, 1));
    store.push(notification("error", NotificationPriority::Error, 2));
    store.push(notification("warn", NotificationPriority::Warning, 3));
    assert_eq!(store.items[0].id, "error");
    assert!(!store.items.iter().any(|item| item.message.contains("must-hide")));
    store.push(notification("warn", NotificationPriority::Success, 4));
    assert_eq!(store.items.iter().filter(|item| item.id == "warn").count(), 1);
    store.expire(200);
    assert!(store.items.is_empty());

    let mut exit = ExitFlow::default();
    assert_eq!(exit.press(1_000, true), ExitDecision::Interrupt);
    assert_eq!(exit.press(1_500, true), ExitDecision::Exit);
    assert_eq!(exit.press(3_000, false), ExitDecision::ArmExit);
    assert_eq!(exit.press(3_900, false), ExitDecision::ArmExit);

    let mut state = TuiState::default();
    state.status.session_title = "Session\u{1b}]bad".to_string();
    state.status.model_name = Some("gpt-5.6".to_string());
    let status = builtin_status_line(&state);
    assert!(status.contains("gpt-5.6"));
    let title = osc_title(&state);
    assert_eq!(title.matches('\u{1b}').count(), 1, "payload controls are sanitized");
    assert_eq!(progress_bar(2, 4, 6), "━━━───");

    let startup =
        startup_notifications(Some("gateway degraded"), Err("update offline".to_string()), 10);
    assert_eq!(startup.len(), 1, "update-check failures never block startup");
    assert!(startup[0].persistent);
}
