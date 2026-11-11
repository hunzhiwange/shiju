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
        .subscription(App::subscription)
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
    let base = iced::widget::stack![base, app::components::toast::view(app)].into();
    app::components::system_settings_companion::size_modal::with_size_modal(app, base)
}
