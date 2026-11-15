use super::*;
#[test]
fn history_events_preserve_navigation_and_block_unprojected_actions() {
    assert!(matches!(history_event::<()>(3, HistoryEvent::Blocked), HistoryAction::None));
    assert!(matches!(history_event::<()>(3, HistoryEvent::ToggleFile), HistoryAction::SelectFile(3)));
    assert!(matches!(history_event::<()>(3, HistoryEvent::OpenMenu), HistoryAction::OpenMenu(3)));
    assert!(matches!(history_event::<()>(3, HistoryEvent::CloseMenu), HistoryAction::CloseMenu));
    assert!(matches!(history_event::<()>(3, HistoryEvent::PreviewFile), HistoryAction::PreviewFile(3)));
    assert!(matches!(history_event::<()>(3, HistoryEvent::CopyFile), HistoryAction::CopyFile(3)));
    assert!(matches!(history_event(3, HistoryEvent::Browse(7)), HistoryAction::Browse(7)));
}
