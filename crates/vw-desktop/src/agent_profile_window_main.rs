//! Explicit native window acceptance for agent profile editing, isolated from user bootstrap.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
#[cfg(not(target_arch = "wasm32"))]
mod desktop_update_helper;
mod fonts;
use app::message::settings::{AgentsMessage, SettingsMessage};
use app::{App, Message};
use iced::{Element, Task, Theme};

fn main() {
    app::config::install_window_test_bootstrap(9);
    iced::application(
        || {
            let (mut app, _) = App::new();
            app.agents_settings.config_ready = true;
            app.agents_settings.loading = false;
            app.agents_settings.entries = desktop_settings_agents::acceptance::entries();
            app.agents_settings.selected_agent = "acp-codex".into();
            app.session_state.acp_agents = vec!["codex".into(), "claude".into()];
            app.workspace_state.recent_projects = vec![
                "/tmp/agent-acceptance-a".into(),
                "/tmp/agent-acceptance-b".into(),
            ];
            let _ = app::message::settings::update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::ProfileOpen("acp-codex".into())),
            );
            (app, Task::none())
        },
        |app: &mut App, message: Message| {
            app::message::settings::update(
                app,
                match message {
                    Message::Settings(value) => value,
                    _ => return Task::none(),
                },
            )
        },
        view,
    )
    .title("Agent Profile Acceptance — isolated gateway port 9")
    .theme(|_: &App| {
        if std::env::args().any(|arg| arg == "--light")
            || std::env::var("SHIJU_PROFILE_ACCEPTANCE_THEME").as_deref() == Ok("light")
        {
            Theme::Light
        } else {
            Theme::Dark
        }
    })
    .window_size((1280.0, 940.0))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}

fn view(app: &App) -> Element<'_, Message> {
    iced::widget::stack![
        app::components::system_settings_agents::view(app),
        app::components::toast::view(app)
    ]
    .into()
}
