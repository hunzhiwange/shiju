//! 验证心跳设置的启停、间隔边界、投递配置与帮助覆盖层。
//! 空输入和极限间隔用于确认调度配置仍被分组到独立面板。

use super::*;

use iced::advanced::widget::Tree;
use iced::widget::text;

// 心跳页面基线关闭调度，各用例显式开启所需配置以突出条件分支。
struct Fixture { heartbeat_settings: crate::app_init::state::HeartbeatSettingsState, theme: iced::Theme }
impl Fixture {
 fn data(&self) -> ViewData<'_> { ViewData { heartbeat_settings: &self.heartbeat_settings, desktop_locale: config_ui::DesktopLocale::default(), app_theme: &self.theme, last_copied_code_hash: None, help_content: |_,body,_,_| text(body).into() } }
}
fn test_app() -> Fixture { Fixture { heartbeat_settings: Default::default(), theme: iced::Theme::Dark } }

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn system_settings_heartbeat_tests_are_wired() {
    assert!(module_path!().contains("system_settings_heartbeat_tests"));
}

#[test]
fn building_blocks_return_elements() {
    let app = test_app();
    keep_element(field_row(&app.data(), "标签", "说明", text("control")));
    keep_element(text_row(&app.data(), "消息", "兜底说明", "可选", "", |_| Message::None));
    keep_element(text_row(&app.data(), "目标", "投递通道", "telegram", "tg", |_| Message::None));
}

#[test]
fn view_builds_disabled_enabled_and_error_heartbeat_states() {
    let app = test_app();
    keep_element(view(&app.data()));

    let mut enabled = test_app();
    enabled.heartbeat_settings.enabled = true;
    enabled.heartbeat_settings.interval_minutes = 1440;
    enabled.heartbeat_settings.message_input = "巡检服务状态".to_string();
    enabled.heartbeat_settings.target_input = "telegram".to_string();
    enabled.heartbeat_settings.to_input = "123456".to_string();
    enabled.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());
    keep_element(view(&enabled.data()));
}

#[test]
fn view_handles_boundary_intervals_and_empty_inputs() {
    let mut app = test_app();
    app.heartbeat_settings.interval_minutes = 1;
    keep_element(view(&app.data()));

    app.heartbeat_settings.interval_minutes = 1440;
    keep_element(view(&app.data()));

    app.heartbeat_settings.message_input = String::new();
    app.heartbeat_settings.target_input = String::new();
    app.heartbeat_settings.to_input = String::new();
    keep_element(view(&app.data()));
}

#[test]
fn overlays_return_base_or_help_modal_for_heartbeat() {
    let app = test_app();
    keep_element(view_overlays(&app.data(), text("dialog").into(), |m| m));

    let mut with_help = test_app();
    with_help.heartbeat_settings.show_help_modal = true;
    keep_element(view_overlays(&with_help.data(), text("dialog").into(), |m| m));
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
    keep_element(view(&app.data()));
}

#[test]
fn view_groups_schedule_and_delivery_controls_into_distinct_panels() {
    let app = test_app();
    let tree = Tree::new(view(&app.data()).as_widget());

    assert_eq!(tree.children.len(), 5);
    assert_eq!(tree.children[2].children.len(), 3);
    assert_eq!(tree.children[4].children.len(), 5);
}
