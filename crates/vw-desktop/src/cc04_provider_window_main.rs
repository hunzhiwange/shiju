//! Explicit isolated provider list acceptance; never installs production bootstrap.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
use app::message::app_providers::list::{ListMessage, message};
use app::{App, Message};
use iced::{Element, Task, Theme};
use vw_api_app_identity::AppId;
fn main() {
    let case = std::env::var("SHIJU_CC04_WINDOW_CASE").expect("window case");
    let (app, native_form) = match case.as_str() {
        "list" | "edit" | "endpoints" | "icons" | "universal" | "snippets" | "preview" | "env"
        | "mode" | "paths" => (AppId::Claude, false),
        "claude" => (AppId::Claude, true),
        "codex" => (AppId::Codex, true),
        "gemini" => (AppId::Gemini, true),
        "grok" => (AppId::GrokBuild, true),
        "opencode" => (AppId::OpenCode, true),
        "openclaw" => (AppId::OpenClaw, true),
        "hermes" => (AppId::Hermes, true),
        "pi" => (AppId::Pi, true),
        "omp" => (AppId::Omp, true),
        "mcode" => (AppId::Mcode, true),
        "agy" => (AppId::Agy, true),
        "desktop" => (AppId::ClaudeDesktop, true),
        "dsh" => (AppId::Dsh, true),
        "kimi" => (AppId::Kimi, true),
        _ => panic!("unknown window case"),
    };
    let edit = case == "edit";
    let theme = match std::env::var("SHIJU_CC04_WINDOW_THEME").as_deref() {
        Ok("dark") => Theme::Dark,
        Ok("light") => Theme::Light,
        _ => panic!("unknown theme"),
    };
    let (port, stop, server) = fixture(native_form, app, &case);
    app::config::install_window_test_bootstrap(port);
    iced::application(
        move || {
            let (mut state, _) = App::new();
            state.apps_catalog.host = "fixture-host".into();
            state.apps_catalog.token = "fake-host-token".into();
            let task =
                app::message::app_providers::list::update(&mut state, ListMessage::Open(app));
            if edit {
                state.app_providers.new_draft = Some(desktop_provider_apps::acceptance::create_draft());
            }
            (state, task)
        },
        |state: &mut App, event: Message| match event {
            Message::Settings(value) => app::message::settings::update(state, value),
            _ => Task::none(),
        },
        view,
    )
    .title(match case.as_str() {
        "codex" => "Codex 供应商隔离验收",
        "gemini" => "Gemini CLI 供应商隔离验收",
        "grok" => "Grok Build 供应商隔离验收",
        "opencode" => "OpenCode 供应商隔离验收",
        "openclaw" => "OpenClaw 供应商隔离验收",
        "hermes" => "Hermes 供应商隔离验收",
        "pi" => "Pi 供应商隔离验收",
        "omp" => "OMP 供应商隔离验收",
        "mcode" => "Mcode 供应商隔离验收",
        "agy" => "Agy 供应商隔离验收",
        "desktop" => "Desktop 供应商隔离验收",
        "endpoints" => "端点与请求选项隔离验收",
        "icons" => "供应商图标库隔离验收",
        "universal" => "统一供应商投影隔离验收",
        "snippets" => "公共配置片段隔离验收",
        "preview" => "配置预览与冲突确认隔离验收",
        "env" => "环境诊断隔离验收",
        "mode" => "供应商模式隔离验收",
        "paths" => "配置目录隔离验收",
        "dsh" => "Dsh 供应商隔离验收",
        "kimi" => "Kimi 供应商隔离验收",
        "claude" => "Claude Code 供应商隔离验收",
        _ => "供应商基础编辑隔离验收",
    })
    .theme(move |_: &App| theme.clone())
    .window_size((1000., 820.))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    server.join().unwrap();
}
fn view(state: &App) -> Element<'_, Message> {
    iced::widget::stack![
        app::components::app_providers::list::overlay(
            &state.app_providers,
            iced::widget::container(iced::widget::text("隔离 HTTP SDK 验收"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        ),
        app::components::toast::view(state)
    ]
    .into()
}
use desktop_provider_apps::acceptance::fixture;
