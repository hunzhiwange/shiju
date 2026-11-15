use iced::{Color, Theme};
use super::todo_panel::*;
#[test]
fn todo_id_display_trims_numeric_suffix_and_preserves_non_numeric_values() {
    assert_eq!(todo_id_display(" task_42 "), "42");
    assert_eq!(todo_id_display("story_task_0007"), "0007");
    assert_eq!(todo_id_display("42"), "42");
    assert_eq!(todo_id_display("task_alpha"), "task_alpha");
    assert_eq!(todo_id_display(""), "");
}
#[test]
fn todo_id_display_preserves_empty_suffix_and_non_ascii_digits() {
    assert_eq!(todo_id_display(" task_ "), "task_");
    assert_eq!(todo_id_display("task_１２"), "task_１２");
}
#[test]
fn completed_task_text_stays_readable_in_dark_theme() {
    let color = todo_task_text_color(&Theme::Dark, true);

    assert_eq!(color.r, Theme::Dark.palette().text.r);
    assert_eq!(color.g, Theme::Dark.palette().text.g);
    assert_eq!(color.b, Theme::Dark.palette().text.b);
    assert!(color.a >= 0.78);
}
#[test]
fn task_text_colors_cover_incomplete_and_light_completed_states() {
    assert_eq!(todo_task_text_color(&Theme::Dark, false), Theme::Dark.palette().text);
    assert_eq!(todo_task_text_color(&Theme::Light, false), Theme::Light.palette().text);
    assert_eq!(
        todo_task_text_color(&Theme::Light, true),
        Theme::Light.palette().text.scale_alpha(0.78)
    );
}
#[test]
fn status_pill_colors_cover_completed_running_and_pending_states() {
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, true, false),
        (Color::from_rgb8(46, 194, 126), Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, false, true),
        (Theme::Light.palette().primary, Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, false, false),
        (Color::from_rgb8(71, 85, 105), Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Dark, false, false),
        (Color::from_rgb8(96, 165, 250), Color::WHITE)
    );
}
