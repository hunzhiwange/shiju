use super::*;
use super::super::test_support::Fixture;

fn child_count(element: Element<'_, Message>) -> usize {
    element.as_widget().children().len()
}

#[test]
fn task_logs_build_root_and_panel_for_context_menu_variants() {
    let mut app = Fixture::default();
    app.current_line_height = 20.0;
    app.task_board_logs_editor = text_editor::Content::with_text(
        "[12:00:00] first line\n/tmp/demo.txt\nhttps://example.com",
    );

    std::hint::black_box(super::build_task_logs(app.view()));
    assert!(child_count(super::build_task_logs_panel(app.view(), Size::new(960.0, 420.0))) > 0);

    app.task_board_logs_context_menu_open = true;
    app.task_board_logs_context_menu_pos = Some((24.0, 32.0));
    app.task_board_logs_scroll_top_line = 1.0;
    assert!(child_count(super::build_task_logs_panel(app.view(), Size::new(640.0, 260.0))) > 0);
}
