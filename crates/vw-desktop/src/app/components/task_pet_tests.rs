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
        detail_feedback: None,
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
    keep_element(task_pet_context_menu(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::English,
    )));
    keep_element(human_pet_sprite(PET_BEAUTY_HANDLE.clone(), PET_COLLAPSED_SIZE, 4.0));
    keep_element(code_effect(7));
    keep_element(reply_button(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::English),
        1,
    ));
    keep_element(reply_input(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::English),
        "hello",
    ));
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
    assert_eq!(transparent_pet_window_style(&Theme::Dark).text_color, Some(Color::WHITE));
    assert_eq!(task_card_style(&Theme::Dark).text_color, Some(Theme::Dark.palette().text));

    assert_ne!(
        background_color(tiny_action_button_style(&Theme::Dark, button::Status::Active)),
        background_color(tiny_action_button_style(&Theme::Dark, button::Status::Hovered))
    );
    assert_ne!(
        background_color(reply_button_style(&Theme::Dark, button::Status::Active)),
        background_color(reply_button_style(&Theme::Dark, button::Status::Pressed))
    );

    let active = reply_input_style(&Theme::Dark, text_input::Status::Active);
    let focused =
        reply_input_style(&Theme::Dark, text_input::Status::Focused { is_hovered: false });
    assert_ne!(active.border.color, focused.border.color);
}

async fn mini_click(app: &App, point: iced::Point) -> Vec<Message> {
    mini_interact(
        app,
        point,
        &[
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        ],
    )
    .await
}

async fn mini_interact(app: &App, point: iced::Point, events: &[iced::Event]) -> Vec<Message> {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::mouse;
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui =
        UserInterface::build(window(app), app.task_pet_window_size(), Cache::new(), &mut renderer);
    for theme in [Theme::Light, Theme::Dark] {
        ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
        let size = app.task_pet_window_size();
        let pixels = renderer.screenshot(
            iced::Size::new(size.width as u32, size.height as u32),
            1.0,
            Color::TRANSPARENT,
        );
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "Mini must render visible controls"
        );
        if let Some(directory) = std::env::var_os("SHIJU_MINI_TASK3_SCREENSHOTS")
            .or_else(|| std::env::var_os("SHIJU_MINI_TASK2_SCREENSHOTS"))
        {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            ::image::save_buffer(
                directory.join(format!(
                    "{:?}-{}-{}-reply{}-{theme}.png",
                    app.task_pet_avatar_kind,
                    app.task_pet_items.len(),
                    app.task_pet_collapsed,
                    app.task_pet_reply_request_id.is_some()
                )),
                &pixels,
                size.width as u32,
                size.height as u32,
                ::image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

#[tokio::test]
async fn mini_companion_toolbar_actions_do_not_start_drag() {
    let app = test_app();
    let messages = mini_click(&app, iced::Point::new(65.0, 127.0)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::View(message::ViewMessage::TaskPetWriteMessage)]
    ));
    let messages = mini_click(&app, iced::Point::new(116.0, 127.0)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::View(message::ViewMessage::TaskPetToggleCollapsed)]
    ));
}

#[tokio::test]
async fn mini_companion_none_renders_toolbar() {
    let mut app = test_app();
    app.task_pet_avatar_kind = TaskPetAvatarKind::None;
    let messages = mini_click(&app, iced::Point::new(65.0, 127.0)).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::View(message::ViewMessage::TaskPetWriteMessage)]
    ));
    assert!(mini_click(&app, iced::Point::new(80.0, 45.0)).await.is_empty());
}

#[tokio::test]
async fn mini_companion_stack_has_bounded_depth() {
    let mut app = test_app();
    let empty_size = app.task_pet_window_size();
    for count in [1, 3, 20] {
        app.task_pet_items =
            (1..=count).rev().map(|id| task_item(id, TaskPetStatus::Running)).collect();
        let size = app.task_pet_window_size();
        assert!(size.width >= 320.0);
        assert!(size.height > empty_size.height);
        assert!(size.height <= empty_size.height + 96.0);
        let messages = mini_click(&app, iced::Point::new(70.0, 180.0)).await;
        assert!(messages.iter().any(|m| matches!(m, Message::View(message::ViewMessage::TaskPetItemClicked(id)) if *id == count)));
        // Exposed rear layers are decorative: they must not navigate or intercept clicks.
        let messages = mini_click(&app, iced::Point::new(70.0, size.height - 18.0)).await;
        if count > 1 {
            assert!(
                !messages.iter().any(|m| matches!(
                    m,
                    Message::View(message::ViewMessage::TaskPetItemClicked(_))
                ))
            );
        }
    }
    let many_size = app.task_pet_window_size();
    app.task_pet_items.truncate(3);
    assert_eq!(app.task_pet_window_size(), many_size);
    let front = app.task_pet_items[0].request_id;
    app.open_task_pet_reply(front);
    app.task_pet_reply_input = "请继续处理中文任务".into();
    let messages = mini_click(&app, iced::Point::new(280.0, 256.0)).await;
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::View(message::ViewMessage::TaskPetReplySubmit)))
    );
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, Message::View(message::ViewMessage::TaskPetItemClicked(_))))
    );
}

#[tokio::test]
async fn mini_companion_expanded_cards_keep_actions_and_scroll() {
    use iced::{Event, mouse};
    let mut app = test_app();
    app.task_pet_collapsed = false;
    app.task_pet_expand_progress = 1.0;
    assert_eq!(app.task_pet_window_size().height, 168.0);
    for count in [1, 3] {
        app.task_pet_items = (1..=count).map(|id| task_item(id, TaskPetStatus::Running)).collect();
        let messages = mini_click(&app, iced::Point::new(70.0, 180.0)).await;
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::View(message::ViewMessage::TaskPetItemClicked(1))))
        );
    }
    app.task_pet_items = (1..=20)
        .map(|id| {
            task_item(
                id,
                if id % 2 == 0 { TaskPetStatus::Completed } else { TaskPetStatus::Running },
            )
        })
        .collect();
    app.task_pet_hovered_request_id = Some(1);
    for (point, expected) in [
        (iced::Point::new(30.0, 216.0), 0),
        (iced::Point::new(310.0, 216.0), 1),
        (iced::Point::new(70.0, 260.0), 2),
    ] {
        let messages = mini_click(&app, point).await;
        assert!(
            messages.iter().any(|m| match expected {
                0 => matches!(m, Message::View(message::ViewMessage::TaskPetRemove(1))),
                1 => matches!(m, Message::View(message::ViewMessage::TaskPetReplyPressed(1))),
                _ => matches!(m, Message::View(message::ViewMessage::TaskPetItemClicked(2))),
            }),
            "missing action {expected}: {messages:?}"
        );
    }
    let messages = mini_interact(
        &app,
        iced::Point::new(70.0, 180.0),
        &[
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -2000.0 },
            }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
    .await;
    assert!(
        messages.iter().any(
            |m| matches!(m, Message::View(message::ViewMessage::TaskPetItemClicked(id)) if *id > 3)
        ),
        "scroll should expose later cards: {messages:?}"
    );
    app.open_task_pet_reply(1);
    app.task_pet_reply_input = "请继续处理中文长标题对应的任务".into();
    mini_click(&app, iced::Point::new(5.0, 5.0)).await;
}
