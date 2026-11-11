//! Explicit native companion acceptance; all gateway traffic is isolated to loopback port 9.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;

use app::{App, Message};
use iced::{Element, Task, Theme};

fn main() -> iced::Result {
    app::config::install_window_test_bootstrap(9);
    iced::daemon(boot, update, view)
        .title(|_: &App, _| "迷你伙伴原生验收 — 隔离网关".to_owned())
        .theme(App::theme_for_window)
        .style(app::App::window_style)
        .subscription(subscription)
        .settings(iced::Settings {
            fonts: fonts::load_all(),
            default_font: iced::Font::with_name("PingFang SC"),
            ..Default::default()
        })
        .run()
}

fn boot() -> (App, Task<Message>) {
    let (mut app, _) = App::new(); // Discard startup tasks: never start a real agent.
    app.app_theme =
        if std::env::args().any(|arg| arg == "--light") { Theme::Light } else { Theme::Dark };
    let count = argument("--tasks=").unwrap_or(3);
    assert!([0, 1, 3, 20].contains(&count), "--tasks must be 0, 1, 3 or 20");
    let scale = argument::<f32>("--scale=").unwrap_or(1.0);
    assert!([0.5, 1.0, 2.0].contains(&scale), "--scale must be 0.5, 1 or 2");
    app.task_pet_scale = scale;
    app.task_pet_items = (0..count)
        .map(|index| app::state::TaskPetItem {
            request_id: index as u64 + 1,
            session_id: format!("companion-acceptance-{index}"),
            title: format!("中文原生渲染验收任务 {}：检查动态长标题与叠层卡片边界", index + 1),
            detail: "正在读取上下文并整理结果；完成后可以展开详情、关闭卡片或打开回复。".into(),
            detail_feedback: None,
            project: Some("隔离验收项目".into()),
            status: if index % 2 == 0 {
                app::state::TaskPetStatus::Running
            } else {
                app::state::TaskPetStatus::Completed
            },
            last_click_at: None,
        })
        .collect();
    let (id, open) = iced::window::open(iced::window::Settings {
        size: iced::Size::new(
            if std::env::args().any(|arg| arg == "--narrow") { 420.0 } else { 980.0 },
            780.0,
        ),
        ..Default::default()
    });
    app.register_window_ids(id, None);
    app.task_pet_visible = true;
    let mini = app::message::view::sync_task_pet_window(&mut app);
    (app, Task::batch([open.map(|_| Message::None), mini]))
}

fn argument<T: std::str::FromStr>(prefix: &str) -> Option<T> {
    std::env::args().find_map(|arg| {
        arg.strip_prefix(prefix)
            .map(|value| value.parse().unwrap_or_else(|_| panic!("invalid {prefix} argument")))
    })
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    if scripted() && matches!(&message, Message::None) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static TICKS: AtomicUsize = AtomicUsize::new(0);
        let tick = TICKS.fetch_add(1, Ordering::Relaxed);
        if tick % 8 == 7 {
            return scripted_scene(app, tick / 8);
        }
    }
    if matches!(
        &message,
        Message::View(app::message::ViewMessage::GlobalKeyPressed(
            iced::keyboard::Key::Named(iced::keyboard::key::Named::F12),
            _
        ))
    ) {
        let stamp =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        return Task::batch(
            [("settings", app.main_window_id), ("mini", app.task_pet_window_id)]
                .into_iter()
                .filter_map(|(name, id)| {
                    id.map(|id| {
                        iced::window::screenshot(id).map(move |frame| {
                            let directory = std::path::Path::new("/tmp/shiju-companion-native");
                            std::fs::create_dir_all(directory)
                                .expect("create native evidence directory");
                            let path = directory.join(format!("{stamp}-{name}.png"));
                            image::save_buffer(
                                &path,
                                &frame.rgba,
                                frame.size.width,
                                frame.size.height,
                                image::ColorType::Rgba8,
                            )
                            .expect("save native evidence frame");
                            eprintln!("native frame: {}", path.display());
                            Message::None
                        })
                    })
                }),
        );
    }
    // Native acceptance shortcuts publish the same messages as the visible production controls.
    if let Message::View(app::message::ViewMessage::GlobalKeyPressed(
        iced::keyboard::Key::Named(key),
        _,
    )) = &message
    {
        use app::message::ViewMessage;
        use iced::keyboard::key::Named;
        let action = match key {
            Named::F2 => Some(ViewMessage::TaskPetAvatarCycle),
            Named::F3 => Some(ViewMessage::TaskPetSizeOpen),
            Named::F4 => Some(ViewMessage::TaskPetSizePreview(0.5)),
            Named::F5 => Some(ViewMessage::TaskPetSizePreview(2.0)),
            Named::F6 => Some(ViewMessage::TaskPetSizeReset),
            Named::F7 => Some(ViewMessage::TaskPetSizeCancel),
            Named::F8 => Some(ViewMessage::TaskPetSizeSave),
            Named::F9 => Some(ViewMessage::TaskPetToggleWindow),
            Named::F10 => Some(ViewMessage::TaskPetToggleCollapsed),
            Named::F11 => return iced::exit(),
            _ => None,
        };
        if let Some(action) = action {
            return app.update(Message::View(action));
        }
    }
    // Keep production UI/window routing; fixture sessions must never dispatch agent requests.
    if matches!(message, Message::Chat(_) | Message::Project(_)) {
        return Task::none();
    }
    app.update(message)
}

fn view(app: &App, window: iced::window::Id) -> Element<'_, Message> {
    if app.task_pet_window_id == Some(window) {
        return app.view_window(window);
    }
    let content = iced::widget::container(app::components::system_settings_companion::content(app))
        .padding([0, 7]);
    let base: Element<'_, Message> = iced::widget::container(
        iced::widget::scrollable(content).direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        )),
    )
    .padding(20)
    .into();
    let modal = app::components::system_settings_companion::size_modal::with_size_modal(app, base);
    iced::widget::stack![modal, app::components::toast::view(app)].into()
}

fn scripted() -> bool {
    std::env::args().any(|arg| arg == "--script")
}

fn subscription(app: &App) -> iced::Subscription<Message> {
    let production = app.subscription();
    if scripted() {
        iced::Subscription::batch([
            production,
            iced::time::every(std::time::Duration::from_millis(200)).map(|_| Message::None),
        ])
    } else {
        production
    }
}

// Drive actual production messages between native frames; never dispatch fixture agent work.
fn scripted_scene(app: &mut App, scene: usize) -> Task<Message> {
    use app::message::ViewMessage as V;
    use app::state::TaskPetAvatarKind as Avatar;
    let committed = argument::<f32>("--scale=").unwrap_or(1.0);
    let preview = if committed == 2.0 { 0.5 } else { 2.0 };
    eprintln!("native scene {scene}");
    let capture = |app: &mut App| {
        update(
            app,
            Message::View(V::GlobalKeyPressed(
                iced::keyboard::Key::Named(iced::keyboard::key::Named::F12),
                iced::keyboard::Modifiers::empty(),
            )),
        )
    };
    let action = match scene {
        0 => return capture(app),
        1 => Some(V::TaskPetAvatarSelected(Avatar::Beauty)),
        2 => {
            assert_eq!(app.task_pet_avatar_kind, Avatar::Beauty);
            return capture(app);
        }
        3 => Some(V::TaskPetAvatarSelected(Avatar::Handsome)),
        4 => {
            assert_eq!(app.task_pet_avatar_kind, Avatar::Handsome);
            return capture(app);
        }
        5 => Some(V::TaskPetAvatarSelected(Avatar::None)),
        6 => {
            assert_eq!(app.task_pet_avatar_kind, Avatar::None);
            return capture(app);
        }
        7 => Some(V::TaskPetSizeOpen),
        8 => {
            assert_eq!(app.task_pet_size_draft, Some(committed));
            return capture(app);
        }
        9 => Some(V::TaskPetSizeCancel),
        10 => Some(V::TaskPetAvatarSelected(Avatar::Robot)),
        11 => Some(V::TaskPetSizeOpen),
        12 => Some(V::TaskPetSizePreview(0.5)),
        13 => {
            assert_eq!(app.task_pet_size_draft, Some(0.5));
            return capture(app);
        }
        14 => Some(V::TaskPetSizePreview(2.0)),
        15 => {
            assert_eq!(app.task_pet_size_draft, Some(2.0));
            return capture(app);
        }
        16 => Some(V::TaskPetSizeReset),
        17 => {
            assert_eq!(app.task_pet_size_draft, Some(1.0));
            return capture(app);
        }
        18 => Some(V::TaskPetSizePreview(preview)),
        19 => Some(V::TaskPetSizeSave),
        20 => {
            assert_eq!(app.task_pet_scale, committed);
            assert_eq!(app.task_pet_size_draft, Some(preview));
            assert!(app.task_pet_size_pending.is_none());
            assert!(app.active_toast.is_some());
            return capture(app);
        }
        21 => Some(V::TaskPetSizeCancel),
        22 => {
            assert!(app.task_pet_size_draft.is_none());
            Some(V::TaskPetHideWindow)
        }
        23 => {
            assert!(!app.task_pet_visible);
            assert!(app.task_pet_window_id.is_none());
            return capture(app);
        }
        24 => Some(V::TaskPetShowWindow),
        25 => {
            assert!(app.task_pet_visible);
            assert!(app.task_pet_window_id.is_some());
            return capture(app);
        }
        26 => Some(V::TaskPetToggleCollapsed),
        27 => {
            assert!(!app.task_pet_collapsed);
            return capture(app);
        }
        28 => return iced::widget::operation::snap_to_end(app.task_pet_scroll_id.clone()),
        29 => return capture(app),
        30 => {
            return iced::widget::operation::snap_to(
                app.task_pet_scroll_id.clone(),
                iced::widget::operation::RelativeOffset::START,
            );
        }
        31 if !app.task_pet_items.is_empty() => Some(V::TaskPetReplyPressed(1)),
        32 if !app.task_pet_items.is_empty() => {
            Some(V::TaskPetReplyInputChanged("中文原生回复输入验收，禁止发送真实代理。".into()))
        }
        33 => return capture(app),
        34 if !app.task_pet_items.is_empty() => Some(V::TaskPetReplySubmit),
        35 => {
            assert!(app.task_pet_reply_request_id.is_none());
            return Task::batch(
                [
                    (app.main_window_id, iced::Point::new(150.0, 100.0)),
                    (app.task_pet_window_id, iced::Point::new(600.0, 180.0)),
                ]
                .into_iter()
                .filter_map(|(id, position)| id.map(|id| iced::window::move_to(id, position))),
            );
        }
        36 => return capture(app),
        37 => Some(V::TaskPetContextMenuOpened(15.0, 15.0)),
        38 => {
            assert!(app.task_pet_context_menu_pos.is_some());
            return capture(app);
        }
        39 => Some(V::TaskPetContextMenuClosed),
        40 => Some(V::TaskPetToggleCollapsed),
        41 => {
            assert!(app.task_pet_collapsed);
            return capture(app);
        }
        42 => return iced::exit(),
        _ => None,
    };
    action.map_or_else(Task::none, |action| app.update(Message::View(action)))
}
