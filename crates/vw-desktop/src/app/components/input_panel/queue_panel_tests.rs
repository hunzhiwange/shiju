use super::queue_panel::{
    format_queue_time, is_dark_theme, queue_item_label, queue_item_meta_label, queue_item_style,
    queue_move_availability, queue_panel, queue_panel_style,
};
use crate::app::QueueItem;
use crate::app::state::{AcpHistoryReplayMode, ChatSendBehavior};
use iced::{Background, Color, Length, Theme};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected a solid color background"),
    }
}

fn queue_item(query: &str, created_ms: u64) -> QueueItem {
    QueueItem {
        created_ms,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

#[test]
fn format_queue_time_formats_epoch_millis_and_rejects_out_of_range_values() {
    assert_eq!(format_queue_time(0), "1970-01-01 00:00:00");
    assert_eq!(format_queue_time(999), "1970-01-01 00:00:00");
    assert_eq!(format_queue_time(1_000), "1970-01-01 00:00:01");
    assert_eq!(format_queue_time(u64::MAX), "");
}

#[test]
fn queue_item_label_uses_trimmed_first_line_and_preserves_empty_first_line() {
    assert_eq!(queue_item_label("  first line  \nsecond line"), "first line");
    assert_eq!(queue_item_label("\nsecond line"), "");
    assert_eq!(queue_item_label("   "), "");
}

#[test]
fn queue_item_label_preserves_exact_limit_without_ellipsis() {
    let exact_limit = "界".repeat(42);

    assert_eq!(queue_item_label(&exact_limit), exact_limit);
}

#[test]
fn queue_item_label_truncates_unicode_by_character_count() {
    let prefix = "界".repeat(42);
    let over_limit = format!("{prefix}尾");

    assert_eq!(queue_item_label(&over_limit), format!("{prefix}…"));
}

#[test]
fn queue_item_label_handles_empty_input_and_ascii_overflow() {
    let exact_limit = "a".repeat(42);
    let over_limit = format!("{exact_limit}b");

    assert_eq!(queue_item_label(""), "");
    assert_eq!(queue_item_label(&exact_limit), exact_limit);
    assert_eq!(queue_item_label(&over_limit), format!("{exact_limit}…"));
}

#[test]
fn queue_item_meta_label_marks_next_item_and_handles_invalid_time() {
    assert_eq!(queue_item_meta_label(0, false, false), "1970-01-01 00:00:00");
    assert_eq!(queue_item_meta_label(0, true, false), "下一条 · 1970-01-01 00:00:00");
    assert_eq!(queue_item_meta_label(0, true, true), "引导 · 下一条 · 1970-01-01 00:00:00");
    assert_eq!(queue_item_meta_label(0, false, true), "引导 · 1970-01-01 00:00:00");
    assert_eq!(queue_item_meta_label(u64::MAX, false, false), "");
    assert_eq!(queue_item_meta_label(u64::MAX, true, false), "下一条");
    assert_eq!(queue_item_meta_label(u64::MAX, false, true), "引导");
}

#[test]
fn queue_move_availability_disables_only_boundary_directions() {
    assert_eq!(queue_move_availability(0, 0), (false, false));
    assert_eq!(queue_move_availability(0, 1), (false, false));
    assert_eq!(queue_move_availability(0, 3), (false, true));
    assert_eq!(queue_move_availability(1, 3), (true, true));
    assert_eq!(queue_move_availability(2, 3), (true, false));
    assert_eq!(queue_move_availability(9, 3), (false, false));
}

#[test]
fn queue_theme_detection_distinguishes_light_and_dark() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn queue_item_style_changes_for_next_item_and_theme() {
    let next_dark = queue_item_style(&Theme::Dark, true);
    let waiting_dark = queue_item_style(&Theme::Dark, false);
    let next_light = queue_item_style(&Theme::Light, true);
    let waiting_light = queue_item_style(&Theme::Light, false);

    assert_eq!(background_color(next_dark.background), Color::from_rgba8(27, 35, 50, 0.98));
    assert_eq!(background_color(waiting_dark.background), Color::from_rgba8(25, 27, 32, 0.96));
    assert_eq!(background_color(next_light.background), Color::from_rgba8(242, 247, 255, 1.0));
    assert_eq!(background_color(waiting_light.background), Color::from_rgba8(251, 252, 253, 1.0));
    assert_eq!(next_dark.border.color, Color::from_rgba8(93, 140, 255, 0.78));
    assert_eq!(waiting_dark.border.color, Color::from_rgba8(54, 58, 66, 0.94));
    assert_eq!(next_light.border.color, Color::from_rgba8(111, 150, 255, 0.42));
    assert_eq!(
        waiting_light.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.72)
    );
    assert_eq!(next_dark.border.width, 1.0);
    assert_eq!(next_dark.border.radius, 12.0.into());
    assert_eq!(next_dark.shadow.blur_radius, 12.0);
    assert_eq!(next_dark.shadow.color, Color::BLACK.scale_alpha(0.18));
    assert_eq!(next_light.shadow.color, Color::BLACK.scale_alpha(0.04));
}

#[test]
fn queue_panel_style_uses_theme_specific_container_background() {
    let dark = queue_panel_style(&Theme::Dark);
    let light = queue_panel_style(&Theme::Light);

    assert_eq!(background_color(dark.background), Color::from_rgba8(15, 17, 21, 0.96));
    assert_eq!(
        background_color(light.background),
        Theme::Light.extended_palette().background.base.color.scale_alpha(0.94)
    );
    assert_eq!(dark.border.color, Color::from_rgba8(47, 51, 59, 0.96));
    assert_eq!(
        light.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.82)
    );
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.radius, 14.0.into());
    assert_eq!(dark.shadow.blur_radius, 10.0);
    assert_eq!(dark.shadow.color, Color::BLACK.scale_alpha(0.22));
    assert_eq!(light.shadow.color, Color::BLACK.scale_alpha(0.06));
}

#[test]
fn queue_panel_renders_empty_short_and_long_queues() {
    let empty = queue_panel(Vec::new(), false);
    assert_eq!(empty.as_widget().size().width, Length::Fill);

    let mut guided = queue_item("first line\nsecond line", 0);
    guided.send_behavior = ChatSendBehavior::Guide;
    let short = queue_panel(vec![guided], true);
    assert_eq!(short.as_widget().size().width, Length::Fill);

    let long_label = "这是一条很长很长很长很长很长很长很长很长很长的任务标题";
    let long =
        queue_panel(vec![queue_item(long_label, 1_000), queue_item("waiting", 2_000)], false);
    assert_eq!(long.as_widget().size().width, Length::Fill);
}
