//! 验证任务宠物的动画帧、窗口展开状态、控件和任务卡片。
//!
//! 动画帧与方向按周期边界取样，任务卡片覆盖运行、完成、悬停和回复状态。

use super::*;
use crate::app::state::{TaskPetAvatarKind, TaskPetItem, TaskPetStatus};
use crate::app::{App, Message};
use iced::widget::{button, text_input};
use iced::{Background, Color, Element, Theme};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn task_item(request_id: u64, status: TaskPetStatus) -> TaskPetItem {
    TaskPetItem {
        request_id,
        session_id: format!("session-{request_id}"),
        title: "正在处理一个较长的任务标题用于截断".to_string(),
        detail: "正在读取文件并整理上下文，稍后继续执行".to_string(),
        project: Some("vibe-window".to_string()),
        status,
        last_click_at: None,
    }
}

fn background_color(style: button::Style) -> Color {
    match style.background.expect("button style should set background") {
        Background::Color(color) => color,
        other => panic!("expected solid color, got {other:?}"),
    }
}

#[test]
fn truncate_for_card_only_adds_ellipsis_when_needed() {
    assert_eq!(truncate_for_card("short", 10), "short");
    assert_eq!(truncate_for_card("abcdef", 3), "abc...");
    assert_eq!(truncate_for_card("你好世界", 2), "你好...");
}

#[test]
fn frame_helpers_keep_expected_animation_cycles() {
    assert!(one_second_pulse(0));
    assert!(one_second_pulse(1));
    assert!(!one_second_pulse(2));
    assert_eq!(double_speed_frame(4, 5), 1);
    assert_eq!(double_speed_frame(usize::MAX, 5), usize::MAX / 5);
}

#[test]
fn window_builds_collapsed_and_expanded_variants() {
    let mut app = test_app();
    app.task_pet_items = vec![task_item(1, TaskPetStatus::Running)];
    app.task_pet_collapsed = true;
    app.task_pet_expand_target = None;
    keep_element(window(&app));
    keep_element(collapsed_window(&app));

    app.task_pet_collapsed = false;
    app.task_pet_expand_progress = 1.0;
    app.task_pet_hovered_request_id = Some(1);
    keep_element(window(&app));
    keep_element(expanded_window(&app));

    app.task_pet_robot_hovered = true;
    keep_element(expanded_window(&app));
}

#[test]
fn pet_control_covers_avatar_badge_code_and_collapse_controls() {
    let mut app = test_app();
    app.task_pet_items = vec![task_item(1, TaskPetStatus::Running)];
    app.status_animation_frame = 11;
    keep_element(pet_control(&app, PET_COLLAPSED_SIZE, true));
    keep_element(pet_control(&app, PET_EXPANDED_SIZE, false));

    app.task_pet_robot_hovered = true;
    keep_element(pet_control(&app, PET_COLLAPSED_SIZE, true));
    keep_element(pet_control(&app, PET_EXPANDED_SIZE, false));

    app.task_pet_avatar_kind = TaskPetAvatarKind::Beauty;
    keep_element(pet_sprite(&app, PET_EXPANDED_SIZE));
    app.task_pet_avatar_kind = TaskPetAvatarKind::Handsome;
    keep_element(pet_sprite(&app, PET_EXPANDED_SIZE));

    app.task_pet_items = vec![task_item(2, TaskPetStatus::Completed)];
    app.task_pet_avatar_kind = TaskPetAvatarKind::Robot;
    app.task_pet_robot_hovered = false;
    keep_element(pet_control(&app, PET_COLLAPSED_SIZE, true));
}

#[test]
fn robot_sprite_covers_hover_pulse_and_walking_directions() {
    let mut app = test_app();

    app.task_pet_robot_hovered = true;
    app.status_animation_frame = 0;
    keep_element(robot_pet_sprite(&app, PET_COLLAPSED_SIZE));

    app.status_animation_frame = 3;
    keep_element(robot_pet_sprite(&app, PET_COLLAPSED_SIZE));

    app.task_pet_robot_hovered = false;
    app.task_pet_dragging = true;
    app.task_pet_drag_direction = -1;
    app.status_animation_frame = 1;
    keep_element(robot_pet_sprite(&app, PET_COLLAPSED_SIZE));

    app.task_pet_drag_direction = 1;
    app.status_animation_frame = 2;
    keep_element(robot_pet_sprite(&app, PET_COLLAPSED_SIZE));

    std::hint::black_box(walk_pet_handle(PET_WALK_RIGHT_BODY_0, true));
    std::hint::black_box(walk_pet_handle(PET_WALK_RIGHT_BODY_0, false));
}

#[test]
fn task_card_builds_running_completed_hovered_and_replying_states() {
    let mut app = test_app();
    let running = task_item(1, TaskPetStatus::Running);
    let completed = task_item(2, TaskPetStatus::Completed);

    keep_element(task_card(&app, &running));
    keep_element(task_card(&app, &completed));

    app.task_pet_hovered_request_id = Some(1);
    keep_element(task_card(&app, &running));

    app.task_pet_reply_request_id = Some(1);
    app.task_pet_reply_input = "继续执行".to_string();
    keep_element(task_card(&app, &running));
}

#[test]
fn leaf_widgets_and_status_icons_build() {
    keep_element(robot_pet_sprite(&test_app(), PET_COLLAPSED_SIZE));
    keep_element(task_pet_context_menu());
    keep_element(human_pet_sprite(PET_BEAUTY_HANDLE.clone(), PET_COLLAPSED_SIZE, 4.0));
    keep_element(code_effect(7));
    keep_element(avatar_cycle_button());
    keep_element(badge_button(12));
    keep_element(expand_collapse_button(true));
    keep_element(expand_collapse_button(false));
    keep_element(reply_button(1));
    keep_element(reply_input("hello"));
    keep_element(centered_button_label("发送", 12));
    keep_element(task_detail_line("detail"));
    keep_element(completed_status_icon());
    keep_element(running_status_icon(3));
    keep_element(task_status_slot(completed_status_icon()));
}

#[test]
fn human_motion_and_handle_follow_walking_hover_and_work_state() {
    let mut app = test_app();
    app.task_pet_avatar_kind = TaskPetAvatarKind::Beauty;
    app.status_animation_frame = 0;

    assert_eq!(human_motion_lift(&app), 0.0);

    app.task_pet_robot_hovered = true;
    assert_eq!(human_motion_lift(&app), 5.0);

    app.status_animation_frame = 1;
    assert_eq!(human_motion_lift(&app), 2.0);

    app.task_pet_robot_hovered = false;
    app.task_pet_items = vec![task_item(1, TaskPetStatus::Running)];
    app.status_animation_frame = 0;
    assert_eq!(human_motion_lift(&app), 5.0);

    app.status_animation_frame = 2;
    assert_eq!(human_motion_lift(&app), 0.0);

    keep_element(human_pet_sprite(
        human_avatar_handle(
            &app,
            PET_BEAUTY_HANDLE.clone(),
            PET_BEAUTY_LEFT_HANDLE.clone(),
            PET_BEAUTY_WORK_HANDLE.clone(),
        ),
        PET_EXPANDED_SIZE,
        human_motion_lift(&app),
    ));

    app.task_pet_dragging = true;
    app.task_pet_drag_direction = -1;
    keep_element(human_pet_sprite(
        human_avatar_handle(
            &app,
            PET_BEAUTY_HANDLE.clone(),
            PET_BEAUTY_LEFT_HANDLE.clone(),
            PET_BEAUTY_WORK_HANDLE.clone(),
        ),
        PET_EXPANDED_SIZE,
        human_motion_lift(&app),
    ));

    app.task_pet_drag_direction = 1;
    keep_element(human_pet_sprite(
        human_avatar_handle(
            &app,
            PET_BEAUTY_HANDLE.clone(),
            PET_BEAUTY_LEFT_HANDLE.clone(),
            PET_BEAUTY_WORK_HANDLE.clone(),
        ),
        PET_EXPANDED_SIZE,
        human_motion_lift(&app),
    ));
}

#[test]
fn style_functions_distinguish_default_hover_and_focus_states() {
    assert_eq!(expanded_pet_window_style(&Theme::Dark).text_color, Some(Color::WHITE));
    assert_eq!(transparent_pet_window_style(&Theme::Dark).text_color, Some(Color::WHITE));
    assert_eq!(task_card_style(&Theme::Dark).text_color, Some(Color::WHITE));

    assert_ne!(
        background_color(tiny_action_button_style(&Theme::Dark, button::Status::Active)),
        background_color(tiny_action_button_style(&Theme::Dark, button::Status::Hovered))
    );
    assert_ne!(
        background_color(reply_button_style(&Theme::Dark, button::Status::Active)),
        background_color(reply_button_style(&Theme::Dark, button::Status::Pressed))
    );
    assert_ne!(
        background_color(borderless_icon_button_style(&Theme::Dark, button::Status::Active)),
        background_color(borderless_icon_button_style(&Theme::Dark, button::Status::Hovered))
    );
    assert_ne!(
        background_color(green_badge_button_style(&Theme::Dark, button::Status::Active)),
        background_color(green_badge_button_style(&Theme::Dark, button::Status::Pressed))
    );

    let active = reply_input_style(&Theme::Dark, text_input::Status::Active);
    let focused =
        reply_input_style(&Theme::Dark, text_input::Status::Focused { is_hovered: false });
    assert_ne!(active.border.color, focused.border.color);
}
