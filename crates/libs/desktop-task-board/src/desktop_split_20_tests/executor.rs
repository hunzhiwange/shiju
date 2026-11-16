use super::ui::*;
use crate::message::TaskBoardMessage;
use iced::{Point, Size, Theme, mouse};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_35_executor_overlay_scroll_hover_and_original_values() {
    let mut app = crate::slice_view_fixture::Fixture::default();
    let mut renderer = renderer().await;
    app.task_board_bulk_acp_agent = Some("codex".into());
    app.acp_agents = ["codex".into(), "claude".into()]
        .into_iter()
        .chain((0..24).map(|idx| format!("示例執行器/{idx}{}", "測試".repeat(22))))
        .collect();
    app.task_board_bulk_executor_popover = true;
    let size = Size::new(420, 480);
    let mut ui = scene(
        crate::panel::executor_selector::build_bulk_executor_selector(app.executor()),
        &mut renderer,
        size,
    );
    let theme = Theme::Dark;
    let idle =
        capture(&mut ui, &mut renderer, &theme, size, "executor-idle", mouse::Cursor::Unavailable);
    let (menu, _) = labels(&mut ui, &renderer).scroll("task_board_executor_selector_scroll");
    assert!(menu.x >= 0.0 && menu.x + menu.width <= 420.0);
    assert!(menu.y >= 38.0 && menu.y + menu.height <= 480.0);
    let codex = Point::new(menu.center_x(), menu.y + 54.0);
    let claude = Point::new(menu.center_x(), menu.y + 92.0);
    let first = hover(&mut ui, &mut renderer, codex);
    let first = capture(&mut ui, &mut renderer, &theme, size, "executor-hover-codex", first);
    let second = hover(&mut ui, &mut renderer, claude);
    let second = capture(&mut ui, &mut renderer, &theme, size, "executor-hover-claude", second);
    assert!(first != idle && second != idle && first != second);
    let outside = hover(&mut ui, &mut renderer, Point::new(410.0, 470.0));
    assert!(capture(&mut ui, &mut renderer, &theme, size, "executor-exit", outside) == idle);
    assert!(matches!(
        click(&mut ui, &mut renderer, claude).as_slice(),
        [TaskBoardMessage::BulkExecutorSelected(Some(value))] if value == "claude"
    ));
    assert!(wheel(&mut ui, &mut renderer, claude, -600.0).is_empty());
    assert!(labels(&mut ui, &renderer).scroll("task_board_executor_selector_scroll").1.y > 0.0);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "executor-scrolled",
        mouse::Cursor::Unavailable,
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(410.0, 470.0)).as_slice(),
        [TaskBoardMessage::CloseBulkExecutorPopover]
    ));
}
