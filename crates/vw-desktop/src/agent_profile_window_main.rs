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
            app.agents_settings.entries = [
                ("acp-codex", serde_json::json!({"label":"内置 Codex 验收", "description":"中文长描述，验证资料编辑与暗黑主题。".repeat(8), "runtime":{"kind":"acp","backend":"codex"}, "recommended_prompts":["检查代码", "解释变更"], "system_prompt":"规则：先阅读代码，明确说明验证结果。"})),
                ("profile-fixture", serde_json::json!({"label":"自定义验收", "runtime":{"kind":"native"}})),
            ].into_iter().map(|(key, value)| app::state::DelegateAgentSettingsEntry::from_config(key, Some({ let mut full = serde_json::to_value(vw_config_types::agent::DelegateAgentConfig::default()).unwrap(); full.as_object_mut().unwrap().extend(value.as_object().unwrap().clone()); serde_json::from_value(full).unwrap() }))).collect();
            app.agents_settings.selected_agent = "acp-codex".into();
            app.acp_agents = vec!["codex".into()];
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
    .theme(|_: &App| Theme::Dark)
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
