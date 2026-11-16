//! 验证任务宠物的动画帧、窗口展开状态、控件和任务卡片。
//!
//! 动画帧与方向按周期边界取样，任务卡片覆盖运行、完成、悬停和回复状态。

use super::{Fixture, task_item};
use crate::pet_view::PetMessage as Message;
fn window(input: &Fixture) -> iced::Element<'_, Message> {
    crate::task_pet::window(input.view())
}
use crate::pet_state::{TaskPetAvatarKind, TaskPetStatus};

use iced::{Color, Theme};

fn test_app() -> Fixture {
    Fixture::default()
}

async fn mini_click(app: &Fixture, point: iced::Point) -> Vec<Message> {
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

async fn mini_interact(app: &Fixture, point: iced::Point, events: &[iced::Event]) -> Vec<Message> {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::mouse;
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
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
    assert!(matches!(messages.as_slice(), [Message::TaskPetWriteMessage]));
    let messages = mini_click(&app, iced::Point::new(116.0, 127.0)).await;
    assert!(matches!(messages.as_slice(), [Message::TaskPetToggleCollapsed]));
}

#[tokio::test]
async fn mini_companion_none_renders_toolbar() {
    let mut app = test_app();
    app.task_pet_avatar_kind = TaskPetAvatarKind::None;
    let messages = mini_click(&app, iced::Point::new(65.0, 127.0)).await;
    assert!(matches!(messages.as_slice(), [Message::TaskPetWriteMessage]));
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
        assert!(
            messages.iter().any(|m| matches!(m, Message::TaskPetItemClicked(id) if *id == count))
        );
        // Exposed rear layers are decorative: they must not navigate or intercept clicks.
        let messages = mini_click(&app, iced::Point::new(70.0, size.height - 18.0)).await;
        if count > 1 {
            assert!(!messages.iter().any(|m| matches!(m, Message::TaskPetItemClicked(_))));
        }
    }
    let many_size = app.task_pet_window_size();
    app.task_pet_items.truncate(3);
    assert_eq!(app.task_pet_window_size(), many_size);
    let front = app.task_pet_items[0].request_id;
    app.task_pet_reply_request_id = Some(front);
    app.task_pet_hovered_request_id = Some(front);
    app.task_pet_reply_input = "请继续处理中文任务".into();
    let messages = mini_click(&app, iced::Point::new(280.0, 256.0)).await;
    assert!(messages.iter().any(|m| matches!(m, Message::TaskPetReplySubmit)));
    assert!(!messages.iter().any(|m| matches!(m, Message::TaskPetItemClicked(_))));
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
        assert!(messages.iter().any(|m| matches!(m, Message::TaskPetItemClicked(1))));
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
                0 => matches!(m, Message::TaskPetRemove(1)),
                1 => matches!(m, Message::TaskPetReplyPressed(1)),
                _ => matches!(m, Message::TaskPetItemClicked(2)),
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
        messages.iter().any(|m| matches!(m, Message::TaskPetItemClicked(id) if *id > 3)),
        "scroll should expose later cards: {messages:?}"
    );
    app.task_pet_reply_request_id = Some(1);
    app.task_pet_hovered_request_id = Some(1);
    app.task_pet_reply_input = "请继续处理中文长标题对应的任务".into();
    mini_click(&app, iced::Point::new(5.0, 5.0)).await;
}

impl Fixture {
    fn task_pet_window_size(&self) -> iced::Size {
        crate::pet_interaction::task_pet_window_size(
            &self.task_pet_avatar_kind,
            &self.task_pet_expand_progress,
            &self.task_pet_items,
            &self.task_pet_reply_request_id,
            &1.0,
            &None,
        )
    }
}
