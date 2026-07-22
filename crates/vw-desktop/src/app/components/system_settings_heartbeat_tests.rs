use super::*;
use crate::app::App;
use iced::advanced::widget::Tree;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn system_settings_heartbeat_tests_are_wired() {
    assert!(module_path!().contains("system_settings_heartbeat_tests"));
}

#[test]
fn building_blocks_return_elements() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(text_row("消息", "兜底说明", "可选", "", |_| Message::None));
    keep_element(text_row("目标", "投递通道", "telegram", "tg", |_| Message::None));
}

#[test]
fn view_builds_disabled_enabled_and_error_heartbeat_states() {
    let app = test_app();
    keep_element(view(&app));

    let mut enabled = test_app();
    enabled.heartbeat_settings.enabled = true;
    enabled.heartbeat_settings.interval_minutes = 1440;
    enabled.heartbeat_settings.message_input = "巡检服务状态".to_string();
    enabled.heartbeat_settings.target_input = "telegram".to_string();
    enabled.heartbeat_settings.to_input = "123456".to_string();
    enabled.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());
    keep_element(view(&enabled));
}

#[test]
fn view_handles_boundary_intervals_and_empty_inputs() {
    let mut app = test_app();
    app.heartbeat_settings.interval_minutes = 1;
    keep_element(view(&app));

    app.heartbeat_settings.interval_minutes = 1440;
    keep_element(view(&app));

    app.heartbeat_settings.message_input = String::new();
    app.heartbeat_settings.target_input = String::new();
    app.heartbeat_settings.to_input = String::new();
    keep_element(view(&app));
}

#[test]
fn overlays_return_base_or_help_modal_for_heartbeat() {
    let app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    let mut with_help = test_app();
    with_help.heartbeat_settings.show_help_modal = true;
    keep_element(view_overlays(&with_help, text("dialog").into()));
}

#[test]
fn view_with_help_open_still_renders() {
    let mut app = test_app();
    app.heartbeat_settings.show_help_modal = true;
    app.heartbeat_settings.enabled = true;
    app.heartbeat_settings.interval_minutes = 30;
    app.heartbeat_settings.message_input = "巡检".to_string();
    app.heartbeat_settings.target_input = "telegram".to_string();
    app.heartbeat_settings.to_input = "chat123".to_string();
    keep_element(view(&app));
}

#[test]
fn view_groups_schedule_and_delivery_controls_into_distinct_panels() {
    let app = test_app();
    let tree = Tree::new(view(&app).as_widget());

    assert_eq!(tree.children.len(), 5);
    assert_eq!(tree.children[2].children.len(), 3);
    assert_eq!(tree.children[4].children.len(), 5);
}
