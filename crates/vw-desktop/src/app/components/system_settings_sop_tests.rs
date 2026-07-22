use super::{
    approval_timeout_secs_changed, default_execution_mode_changed, execution_mode_options,
    field_row, hint_row, max_concurrent_total_changed, max_finished_runs_changed, number_row,
    rounded_u32, settings_content, sops_dir_changed, sops_dir_pick_folder,
};
use crate::app::Message;
use crate::app::message::settings::{SettingsMessage, SopMessage};
use crate::app::state::SopSettingsState;
use iced::Element;
use iced::widget::text;

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn sop_message(message: Message) -> SopMessage {
    match message {
        Message::Settings(SettingsMessage::Sop(message)) => message,
        _ => panic!("expected SOP settings message"),
    }
}

#[test]
fn execution_mode_options_keep_supported_modes_in_stable_order() {
    let options = execution_mode_options();

    assert_eq!(options.len(), 2);
    assert_eq!(options[0], "supervised");
    assert_eq!(options[1], "autonomous");
}

#[test]
fn rounded_u32_rounds_fractional_number_input_values() {
    for (value, expected) in
        [(0.0, 0), (0.49, 0), (0.5, 1), (1.49, 1), (1.5, 2), (86_400.0, 86_400)]
    {
        assert_eq!(rounded_u32(value), expected);
    }
}

#[test]
fn rounded_u32_saturates_non_finite_and_out_of_range_values() {
    assert_eq!(rounded_u32(f32::NEG_INFINITY), 0);
    assert_eq!(rounded_u32(-1.0), 0);
    assert_eq!(rounded_u32(f32::NAN), 0);
    assert_eq!(rounded_u32(f32::INFINITY), u32::MAX);
}

#[test]
fn directory_callbacks_preserve_empty_relative_and_unicode_paths() {
    for expected in ["", "sops", "../workspace/流程"] {
        match sop_message(sops_dir_changed(expected.to_string())) {
            SopMessage::SopsDirChanged(actual) => assert_eq!(actual, expected),
            _ => panic!("expected directory change message"),
        }
    }

    assert!(matches!(sop_message(sops_dir_pick_folder()), SopMessage::SopsDirPickFolder));
}

#[test]
fn execution_mode_callback_preserves_each_supported_mode() {
    for expected in execution_mode_options() {
        match sop_message(default_execution_mode_changed(expected.clone())) {
            SopMessage::DefaultExecutionModeChanged(actual) => assert_eq!(actual, expected),
            _ => panic!("expected execution mode change message"),
        }
    }
}

#[test]
fn numeric_callbacks_round_values_into_their_distinct_messages() {
    match sop_message(max_finished_runs_changed(99_999.5)) {
        SopMessage::MaxFinishedRunsChanged(value) => assert_eq!(value, 100_000),
        _ => panic!("expected finished runs change message"),
    }

    match sop_message(max_concurrent_total_changed(4.5)) {
        SopMessage::MaxConcurrentTotalChanged(value) => assert_eq!(value, 5),
        _ => panic!("expected concurrent total change message"),
    }

    match sop_message(approval_timeout_secs_changed(299.5)) {
        SopMessage::ApprovalTimeoutSecsChanged(value) => assert_eq!(value, 300),
        _ => panic!("expected approval timeout change message"),
    }
}

#[test]
fn field_and_hint_rows_build_empty_and_populated_content() {
    keep_element(field_row("流程目录", "说明", text("control")));
    keep_element(hint_row(""));
    keep_element(hint_row("人工审批会在开始前请求确认。"));
}

#[test]
fn number_rows_build_supported_minimum_and_maximum_ranges() {
    keep_element(number_row(
        "已完成记录上限",
        "说明",
        0,
        "0 表示不限制",
        0.0,
        100_000.0,
        max_finished_runs_changed,
    ));
    keep_element(number_row(
        "全局并发上限",
        "说明",
        1_000,
        "全局并发运行数",
        1.0,
        1_000.0,
        max_concurrent_total_changed,
    ));
    keep_element(number_row(
        "审批超时",
        "说明",
        86_400,
        "0 表示禁用超时",
        0.0,
        86_400.0,
        approval_timeout_secs_changed,
    ));
}

#[test]
fn settings_content_builds_default_and_boundary_states() {
    let mut settings = SopSettingsState::default();
    keep_element(settings_content(&settings));

    settings.sops_dir_input = "workspace/流程".to_string();
    settings.default_execution_mode = "autonomous".to_string();
    settings.max_finished_runs = 0;
    settings.max_concurrent_total = 1;
    settings.approval_timeout_secs = 0;
    keep_element(settings_content(&settings));

    settings.max_finished_runs = 100_000;
    settings.max_concurrent_total = 1_000;
    settings.approval_timeout_secs = 86_400;
    keep_element(settings_content(&settings));
}
