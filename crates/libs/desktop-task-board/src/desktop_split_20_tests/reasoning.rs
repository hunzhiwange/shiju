use super::ui::*;
use crate::message::TaskBoardMessage;
use config_ui::DesktopLocale;
use iced::keyboard::{Key, key::Named};
use iced::{Point, Size};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_35_preserves_values_and_disabled_states() {
    use vw_api_chat_identity::ReasoningEffortDto;

    let mut app = crate::slice_view_fixture::Fixture::default();
    app.desktop_locale = DesktopLocale::English;
    assert!(
        crate::panel::reasoning_selector::build_bulk_reasoning_effort_selector(app.reasoning())
            .is_none()
    );
    app.task_board_bulk_acp_agent = Some("codex".into());
    app.task_board_bulk_reasoning_effort = Some(ReasoningEffortDto::Low);
    let mut renderer = renderer().await;
    let mut ui = scene(
        crate::panel::reasoning_selector::build_bulk_reasoning_effort_selector(app.reasoning())
            .expect("Codex reasoning options"),
        &mut renderer,
        Size::new(420, 480),
    );

    let point = Point::new(60.0, 18.0);
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(
        matches!(
            key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
            [TaskBoardMessage::UpdateBulkReasoningEffort(Some(ReasoningEffortDto::Xhigh))]
        ),
        "Enter must select the final reasoning option once"
    );
}
