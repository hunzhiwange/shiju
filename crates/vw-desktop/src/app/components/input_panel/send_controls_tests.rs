//! 验证发送控件在运行、排队、停止与禁用状态下的消息和布局。
//! 控件测试分别构造会话运行态和发送策略，确保主操作不会映射到错误消息。

use super::send_controls::{
    bottom_bar, cancel_button, full_access_button, is_dark_theme, permission_access_button_style,
    pool_button, prominent_action_background, prominent_action_foreground, prominent_action_style,
    send_behavior_icon, send_behavior_popover, send_button, subagent_mode_button,
    utility_cluster_style, workflow_mode_button,
};
use super::styles::square_icon_button_style;
use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::state::ChatSendBehavior;
use crate::app::task::TaskSplitMode;
use iced::widget::{Space, button, text};
use iced::{Background, Color, Element, Length, Theme};

fn spacer() -> Element<'static, Message> {
    Space::new().into()
}

fn button_background(style: &iced::widget::button::Style) -> Color {
    match style.background.as_ref() {
        Some(Background::Color(color)) => *color,
        other => panic!("expected solid button background, got {other:?}"),
    }
}

fn container_background(style: &iced::widget::container::Style) -> Color {
    match style.background.as_ref() {
        Some(Background::Color(color)) => *color,
        other => panic!("expected solid container background, got {other:?}"),
    }
}

#[test]
fn send_behavior_icons_match_each_mode() {
    assert_eq!(send_behavior_icon(ChatSendBehavior::Queue), Icon::ListUl);
    assert_eq!(send_behavior_icon(ChatSendBehavior::StopAndSend), Icon::Square);
    assert_eq!(send_behavior_icon(ChatSendBehavior::Guide), Icon::ChatTextFill);
}

#[test]
fn prominent_action_colors_match_dark_and_light_palettes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
    assert_eq!(prominent_action_background(&Theme::Dark), Color::from_rgba8(243, 244, 246, 1.0));
    assert_eq!(prominent_action_background(&Theme::Light), Color::from_rgba8(17, 19, 24, 1.0));
    assert_eq!(prominent_action_foreground(&Theme::Dark), Color::from_rgba8(15, 16, 18, 1.0));
    assert_eq!(prominent_action_foreground(&Theme::Light), Color::WHITE);
}

#[test]
fn prominent_action_style_uses_interaction_colors_when_enabled() {
    let cases = [
        (
            Theme::Dark,
            iced::widget::button::Status::Active,
            Color::from_rgba8(243, 244, 246, 1.0),
            Color::from_rgba8(255, 255, 255, 0.18),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(249, 250, 251, 1.0),
            Color::from_rgba8(255, 255, 255, 0.24),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(231, 233, 237, 1.0),
            Color::from_rgba8(255, 255, 255, 0.24),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Active,
            Color::from_rgba8(17, 19, 24, 1.0),
            Color::from_rgba8(17, 19, 24, 0.12),
            Color::WHITE,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(13, 15, 20, 1.0),
            Color::from_rgba8(17, 19, 24, 0.18),
            Color::WHITE,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(10, 11, 16, 1.0),
            Color::from_rgba8(17, 19, 24, 0.18),
            Color::WHITE,
        ),
    ];

    for (theme, status, expected_background, expected_border, expected_text) in cases {
        let style = prominent_action_style(&theme, status, true);

        assert_eq!(button_background(&style), expected_background);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, expected_border);
        assert_eq!(style.text_color, expected_text);
        assert_eq!(style.shadow.blur_radius, 18.0);
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 8.0));
    }
}

#[test]
fn prominent_action_style_uses_muted_interaction_colors_when_disabled() {
    let cases = [
        (
            Theme::Dark,
            iced::widget::button::Status::Active,
            Color::from_rgba8(24, 25, 29, 0.92),
            Color::from_rgba8(44, 47, 53, 0.95),
            0.36,
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(28, 29, 34, 0.94),
            Color::from_rgba8(44, 47, 53, 0.95),
            0.36,
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(30, 32, 37, 0.96),
            Color::from_rgba8(44, 47, 53, 0.95),
            0.36,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Active,
            Color::from_rgba8(247, 248, 250, 1.0),
            Color::from_rgba8(226, 231, 237, 1.0),
            0.42,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(242, 244, 247, 1.0),
            Color::from_rgba8(226, 231, 237, 1.0),
            0.42,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(238, 241, 245, 1.0),
            Color::from_rgba8(226, 231, 237, 1.0),
            0.42,
        ),
    ];

    for (theme, status, expected_background, expected_border, text_alpha) in cases {
        let style = prominent_action_style(&theme, status, false);

        assert_eq!(button_background(&style), expected_background);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, expected_border);
        assert_eq!(style.text_color, theme.palette().text.scale_alpha(text_alpha));
        assert_eq!(style.shadow.blur_radius, 0.0);
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 0.0));
    }
}

#[test]
fn prominent_action_styles_treat_disabled_status_as_base_state() {
    for theme in [Theme::Dark, Theme::Light] {
        for enabled in [false, true] {
            let active =
                prominent_action_style(&theme, iced::widget::button::Status::Active, enabled);
            let disabled =
                prominent_action_style(&theme, iced::widget::button::Status::Disabled, enabled);

            assert_eq!(button_background(&disabled), button_background(&active));
            assert_eq!(disabled.border.color, active.border.color);
            assert_eq!(disabled.text_color, active.text_color);
        }

        let active = permission_access_button_style(
            &theme,
            iced::widget::button::Status::Active,
            true,
            true,
        );
        let disabled = permission_access_button_style(
            &theme,
            iced::widget::button::Status::Disabled,
            true,
            true,
        );

        assert_eq!(button_background(&disabled), button_background(&active));
        assert_eq!(disabled.border.color, active.border.color);
        assert_eq!(disabled.text_color, active.text_color);
    }
}

#[test]
fn permission_access_style_delegates_to_square_style_when_inactive() {
    for theme in [Theme::Dark, Theme::Light] {
        for enabled in [true, false] {
            for status in [
                iced::widget::button::Status::Active,
                iced::widget::button::Status::Hovered,
                iced::widget::button::Status::Pressed,
                iced::widget::button::Status::Disabled,
            ] {
                let actual = permission_access_button_style(&theme, status, enabled, false);
                let expected = square_icon_button_style(&theme, status, enabled);

                assert_eq!(button_background(&actual), button_background(&expected));
                assert_eq!(actual.border.width, expected.border.width);
                assert_eq!(actual.border.color, expected.border.color);
                assert_eq!(actual.text_color, expected.text_color);
                assert_eq!(actual.shadow.color, expected.shadow.color);
                assert_eq!(actual.shadow.offset, expected.shadow.offset);
                assert_eq!(actual.shadow.blur_radius, expected.shadow.blur_radius);
            }
        }
    }
}

#[test]
fn permission_access_style_highlights_active_state_for_both_themes() {
    let cases = [
        (
            Theme::Dark,
            iced::widget::button::Status::Active,
            Color::from_rgba8(243, 244, 246, 1.0),
            Color::from_rgba8(255, 255, 255, 0.18),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(249, 250, 251, 1.0),
            Color::from_rgba8(255, 255, 255, 0.18),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Dark,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(231, 233, 237, 1.0),
            Color::from_rgba8(255, 255, 255, 0.18),
            Color::from_rgba8(15, 16, 18, 1.0),
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Active,
            Color::from_rgba8(17, 19, 24, 1.0),
            Color::from_rgba8(17, 19, 24, 0.12),
            Color::WHITE,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Hovered,
            Color::from_rgba8(13, 15, 20, 1.0),
            Color::from_rgba8(17, 19, 24, 0.12),
            Color::WHITE,
        ),
        (
            Theme::Light,
            iced::widget::button::Status::Pressed,
            Color::from_rgba8(10, 11, 16, 1.0),
            Color::from_rgba8(17, 19, 24, 0.12),
            Color::WHITE,
        ),
    ];

    for (theme, status, expected_background, expected_border, expected_text) in cases {
        let enabled = permission_access_button_style(&theme, status, true, true);
        let disabled_flag = permission_access_button_style(&theme, status, false, true);

        assert_eq!(button_background(&enabled), expected_background);
        assert_eq!(button_background(&disabled_flag), expected_background);
        assert_eq!(enabled.border.color, expected_border);
        assert_eq!(disabled_flag.border.color, expected_border);
        assert_eq!(enabled.text_color, expected_text);
        assert_eq!(disabled_flag.text_color, expected_text);
        assert_eq!(enabled.shadow.blur_radius, 16.0);
        assert_eq!(disabled_flag.shadow.blur_radius, 16.0);
    }
}

#[test]
fn utility_cluster_style_sets_theme_specific_pill_surface() {
    let dark = utility_cluster_style(&Theme::Dark);
    let light = utility_cluster_style(&Theme::Light);

    assert_eq!(
        container_background(&dark),
        crate::app::theme_catalog::input_surface_color(&Theme::Dark, 0.88)
    );
    assert_eq!(
        container_background(&light),
        crate::app::theme_catalog::input_surface_color(&Theme::Light, 1.0)
    );
    assert_eq!(dark.border.width, 0.0);
    assert_eq!(light.border.width, 0.0);
    assert_eq!(dark.shadow.color, Color::BLACK.scale_alpha(0.12));
    assert_eq!(light.shadow.color, Color::BLACK.scale_alpha(0.035));
    assert_eq!(dark.shadow.offset, iced::Vector::new(0.0, 6.0));
    assert_eq!(light.shadow.offset, iced::Vector::new(0.0, 6.0));
    assert_eq!(dark.shadow.blur_radius, 16.0);
    assert_eq!(light.shadow.blur_radius, 16.0);
}

#[test]
fn pool_button_constructs_disabled_simple_and_task_mode_branches() {
    let disabled_without_task_mode = pool_button(
        false,
        false,
        String::new(),
        "1".into(),
        "auto".into(),
        None,
        TaskSplitMode::FollowSystem,
        Vec::new(),
    );
    let disabled = pool_button(
        false,
        true,
        String::new(),
        "1".into(),
        "auto".into(),
        None,
        TaskSplitMode::FollowSystem,
        Vec::new(),
    );
    let simple = pool_button(
        true,
        false,
        "task".into(),
        "1".into(),
        "auto".into(),
        None,
        TaskSplitMode::FollowSystem,
        Vec::new(),
    );
    let task_mode = pool_button(
        true,
        true,
        "task".into(),
        "5".into(),
        "provider/model".into(),
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High),
        TaskSplitMode::Enabled,
        vec!["sub".into()],
    );

    assert_eq!(disabled_without_task_mode.as_widget().children().len(), 2);
    assert_eq!(disabled.as_widget().children().len(), 2);
    assert_eq!(simple.as_widget().children().len(), 2);
    assert_eq!(task_mode.as_widget().children().len(), 2);
}

#[test]
fn full_access_button_constructs_disabled_enabled_and_active_variants() {
    for element in [
        full_access_button(false, false),
        full_access_button(true, false),
        full_access_button(true, true),
        full_access_button(false, true),
    ] {
        assert_eq!(element.as_widget().children().len(), 2);
    }
}

#[test]
fn workflow_mode_button_constructs_inactive_and_active_variants() {
    for element in [workflow_mode_button(false), workflow_mode_button(true)] {
        assert_eq!(element.as_widget().children().len(), 2);
    }
}

#[test]
fn bottom_bar_places_workflow_after_full_access_and_before_subagent() {
    let controls = include_str!("send_controls.rs");
    let workflow_push = controls.find("bar = bar.push(workflow_btn)").expect("workflow push");
    let subagent_push = controls.find("bar = bar.push(subagent_btn)").expect("subagent push");
    assert!(workflow_push < subagent_push, "bottom bar should place workflow before subagent");
    assert!(
        !controls.contains("bar = bar.push(goal_btn)"),
        "bottom bar must not expose removed goal mode button"
    );
    assert!(
        controls.contains("icon_svg(Icon::Box, SECONDARY_ICON_SIZE)"),
        "subagent mode should use box icon, not the ACP robot icon"
    );
    assert!(
        !controls.contains("subagent_icon: Element<'_, Message> = icon_svg(Icon::Robot"),
        "subagent mode must not reuse ACP default robot icon"
    );

    let mod_source = include_str!("mod.rs");
    assert!(
        mod_source.contains("workflow_btn,\n        subagent_btn,"),
        "input panel should pass workflow then subagent into bottom_bar"
    );
    assert!(!mod_source.contains("goal_btn"), "input panel must not wire goal mode button");
    for tab in ["Agent", "Tools", "Skills"] {
        assert!(
            mod_source.contains(&format!("SessionToolSelectorTab::{tab}")),
            "bottom bar should expose session tab entry for {tab}"
        );
        assert!(
            mod_source.contains("OpenSessionToolSelectorTab"),
            "bottom bar session tabs should open via OpenSessionToolSelectorTab"
        );
    }
    assert!(
        mod_source.contains("SessionToolSelectorTab::Agent => Icon::Hexagon"),
        "agent tab should use hexagon identity icon, not a chat bubble"
    );
    assert!(
        mod_source.contains("SessionToolSelectorTab::Tools => Icon::Terminal"),
        "tools tab should use terminal icon aligned with slash palette"
    );
    assert!(
        mod_source.contains("SessionToolSelectorTab::Skills => Icon::Star"),
        "skills tab should use star icon aligned with slash palette"
    );
    assert!(
        !mod_source.contains("\"会话控制\""),
        "bottom bar should no longer use a single 会话控制 entry button"
    );

    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();
    // permission + workflow + subagent + agent + tools + skills + acp + reasoning + model + fill + utility + actions
    assert_eq!(children.len(), 12);
    assert_eq!(children[10].children.len(), 2);
    assert_eq!(children[11].children.len(), 3);
}

#[test]
fn subagent_mode_button_constructs_inactive_and_active_variants() {
    for element in [subagent_mode_button(false), subagent_mode_button(true)] {
        assert_eq!(element.as_widget().children().len(), 2);
    }
}

#[test]
fn cancel_button_constructs_every_animation_phase() {
    for submit_anim in [0, 2, 4, u8::MAX] {
        let element = cancel_button(submit_anim);

        assert_eq!(element.as_widget().children().len(), 2);
    }
}

#[test]
fn send_button_constructs_idle_and_requesting_state_matrix() {
    let mut app = crate::app::App::new().0;
    let disabled = send_button(&app, false, false, false);
    let enabled = send_button(&app, true, true, false);

    assert_eq!(disabled.as_widget().children().len(), 2);
    assert_eq!(enabled.as_widget().children().len(), 2);
    drop(disabled);
    drop(enabled);

    for behavior in
        [ChatSendBehavior::Queue, ChatSendBehavior::StopAndSend, ChatSendBehavior::Guide]
    {
        app.chat_send_behavior = behavior;
        for show_popover in [false, true] {
            app.show_send_mode_popover = show_popover;
            for can_send in [false, true] {
                let requesting = send_button(&app, can_send, can_send, true);

                assert_eq!(requesting.as_widget().size().width, Length::Shrink);
                assert_eq!(requesting.as_widget().children().len(), 2);
            }
        }
    }
}

#[test]
fn send_behavior_popover_constructs_all_selected_options() {
    for behavior in
        [ChatSendBehavior::Queue, ChatSendBehavior::StopAndSend, ChatSendBehavior::Guide]
    {
        let element = send_behavior_popover(behavior);

        assert_eq!(element.as_widget().size().width, Length::Fixed(320.0));
        assert_eq!(element.as_widget().children().len(), 3);
    }
}

#[test]
fn bottom_bar_places_optional_prefix_utility_and_action_controls() {
    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();

    assert_eq!(normal.as_widget().size().width, Length::Fill);
    assert_eq!(children.len(), 12);
    assert_eq!(children[10].children.len(), 2);
    assert_eq!(children[11].children.len(), 3);
}

#[test]
fn bottom_bar_keeps_minimal_non_task_utility_and_attachment_clusters() {
    let normal = bottom_bar(
        None,
        None,
        None,
        None,
        None,
        None,
        spacer(),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        None,
        None,
        None,
        false,
    );
    let children = normal.as_widget().children();

    assert_eq!(children.len(), 6);
    assert_eq!(children[4].children[0].children.len(), 1);
    assert_eq!(children[5].children.len(), 1);
}

#[test]
fn bottom_bar_task_mode_moves_attachment_into_utility_and_omits_actions() {
    let task_mode = bottom_bar(
        None,
        None,
        None,
        None,
        None,
        None,
        spacer(),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        true,
    );
    let children = task_mode.as_widget().children();

    assert_eq!(task_mode.as_widget().size().width, Length::Fill);
    assert_eq!(children.len(), 5);
    assert_eq!(children[4].children.len(), 3);
}

#[test]
fn bottom_bar_task_mode_without_pool_keeps_usage_and_attachment_only() {
    let task_mode = bottom_bar(
        None,
        None,
        None,
        None,
        None,
        None,
        spacer(),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        None,
        Some(spacer()),
        Some(spacer()),
        true,
    );
    let children = task_mode.as_widget().children();

    assert_eq!(children.len(), 5);
    assert!(!children[4].children.is_empty());
}
