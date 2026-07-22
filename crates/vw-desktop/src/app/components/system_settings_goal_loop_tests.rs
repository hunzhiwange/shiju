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
fn system_settings_goal_loop_tests_are_wired() {
    assert!(module_path!().contains("system_settings_goal_loop_tests"));
}

#[test]
fn building_blocks_return_elements_for_empty_and_populated_controls() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(text_row("间隔", "分钟数", "正整数", "", |_| Message::None));
    keep_element(text_row("目标", "投递目标", "可选", "群聊-研发 🚀", |_| {
        Message::None
    }));
}

#[test]
fn view_builds_disabled_and_enabled_goal_loop_states() {
    let app = test_app();
    keep_element(view(&app));

    let mut enabled = test_app();
    enabled.goal_loop_settings.enabled = true;
    enabled.goal_loop_settings.interval_minutes_input = "60".to_string();
    enabled.goal_loop_settings.step_timeout_secs_input = "300".to_string();
    enabled.goal_loop_settings.max_steps_per_cycle_input = "10".to_string();
    enabled.goal_loop_settings.channel_input = "telegram".to_string();
    enabled.goal_loop_settings.target_input = "chat-id".to_string();
    keep_element(view(&enabled));
}

#[test]
fn view_handles_empty_boundary_and_unvalidated_numeric_inputs() {
    let mut app = test_app();
    app.goal_loop_settings.interval_minutes_input = String::new();
    app.goal_loop_settings.step_timeout_secs_input = String::new();
    app.goal_loop_settings.max_steps_per_cycle_input = String::new();
    app.goal_loop_settings.channel_input = String::new();
    app.goal_loop_settings.target_input = String::new();
    keep_element(view(&app));

    app.goal_loop_settings.interval_minutes_input = "1".to_string();
    app.goal_loop_settings.step_timeout_secs_input = "1".to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = "1".to_string();
    keep_element(view(&app));

    app.goal_loop_settings.interval_minutes_input = "0".to_string();
    app.goal_loop_settings.step_timeout_secs_input = "-1".to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = "not-a-number".to_string();
    keep_element(view(&app));
}

#[test]
fn view_handles_whitespace_long_and_unicode_delivery_inputs() {
    let mut app = test_app();
    app.goal_loop_settings.interval_minutes_input = " 15 ".to_string();
    app.goal_loop_settings.step_timeout_secs_input = u64::MAX.to_string();
    app.goal_loop_settings.max_steps_per_cycle_input = "999999999999999999999999".to_string();
    app.goal_loop_settings.channel_input = "企业微信/telegram".to_string();
    app.goal_loop_settings.target_input = "群聊-研发 🚀".to_string();

    keep_element(view(&app));
}

#[test]
fn view_keeps_basic_and_delivery_fields_in_separate_panels() {
    let app = test_app();
    let tree = Tree::new(view(&app).as_widget());

    assert_eq!(tree.children.len(), 6);
    assert_eq!(tree.children[2].children.len(), 7);
    assert_eq!(tree.children[4].children.len(), 3);
    assert_eq!(tree.children[5].children.len(), 2);
}

#[test]
fn source_wires_each_control_to_goal_loop_message() {
    let source = include_str!("system_settings_goal_loop.rs");

    assert!(source.contains("GoalLoopMessage::EnabledToggled(value)"));
    assert!(source.contains("GoalLoopMessage::IntervalMinutesChanged(value)"));
    assert!(source.contains("GoalLoopMessage::StepTimeoutSecsChanged(value)"));
    assert!(source.contains("GoalLoopMessage::MaxStepsPerCycleChanged(value)"));
    assert!(source.contains("GoalLoopMessage::ChannelChanged("));
    assert!(source.contains("GoalLoopMessage::TargetChanged("));
}
