use super::page_fixture::preferences::*;
use super::page_fixture::*;
use vw_api_app_identity::{
    AppHostPreferences, AppId, AppPreferencesCapabilities, AppPreferencesResult, AppUserPreferences,
};

fn value() -> AppPreferencesResult {
    AppPreferencesResult {
        user: AppUserPreferences {
            app_order: vec!["codex".into()],
            visible_apps: vec![AppId::Codex],
        },
        host: AppHostPreferences::default(),
        restart_required: false,
        capabilities: None,
    }
}

#[test]
fn cc_apps_desktop_preferences_state_message_scopes_and_atomic_domains() {
    let (_, context) = super::lifecycle_app();
    let mut app = AppsCatalogState {
        context: Some(context.clone()),
        host: "中文主机".into(),
        token: "isolated-fixture-application-token".into(),
        ..Default::default()
    };
    app.preferences.saved = Some(value());
    app.preferences.draft = Some(value());
    let route = preferences_route;
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    assert!(app.preferences.busy);
    assert_eq!(app.preferences.saved, Some(value()));
    let generation = app.preferences.generation;
    let mut normalized = value();
    normalized.host.preferred_terminal = Some("server-normalized".into());
    let reply = |host: &str, catalog_generation, result| PreferencesMessage::Reply {
        context: context.clone(),
        host: host.into(),
        catalog_generation,
        generation,
        result,
    };
    let cg = app.generation;
    let _ = route(&mut app, reply("wrong-host", cg, Ok(normalized.clone())));
    assert_eq!(app.preferences.saved, Some(value()));
    let cg = app.generation;
    let _ = route(&mut app, reply("中文主机", cg.wrapping_add(1), Ok(normalized.clone())));
    assert_eq!(app.preferences.saved, Some(value()));
    let _ = route(&mut app, reply("中文主机", cg, Ok(normalized.clone())));
    assert_eq!(app.preferences.saved, Some(normalized));
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    app.input().credentials(Some("other".into()), None);
    assert!(app.preferences.saved.is_none());
    let _ = route(&mut app, reply("中文主机", cg, Ok(value())));
    assert!(app.preferences.saved.is_none());
}

use super::page_fixture as system_settings_apps;
pub(crate) use crate::native_acceptance::help::{HELP_LOCALES, verify_help};
use desktop_i18n::I18n;
use iced::Length;
#[tokio::test]
async fn help_rollout_04_renders_contextual_help() {
    let mut state = AppsCatalogState { host: "fixture-host".into(), ..Default::default() };
    state.preferences.open = true;
    let mut draft = value();
    draft.capabilities = Some(AppPreferencesCapabilities {
        terminals: vec!["fixture-terminal".into()],
        wsl_preferences: true,
    });
    state.preferences.draft = Some(draft);
    state.preferences.error = Some("Fixture preferences error".into());
    for mode in &HELP_LOCALES {
        let messages = verify_help(
            system_settings_apps::view_overlays(
                &state,
                I18n::new(mode.0),
                &mode.1,
                iced::widget::container(system_settings_apps::view(&state, I18n::new(mode.0)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            ),
            "preferences-help",
            mode,
            &[
                ("应用设置", "分别保存用户偏好和主机设置。"),
                ("可见应用与顺序", "显示或隐藏应用，并调整它们在目录中的顺序。"),
                ("主机终端设置", "这些偏好仅用于所选执行主机。"),
                ("WSL", "发行版由所选安装实例决定；只保存各应用的 shell 与 flag。留空表示自动。"),
            ],
            &["执行主机：fixture-host", "Fixture preferences error"],
            "保存主机设置",
        )
        .await;
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Preferences(
                PreferencesMessage::SaveHost
            )))]
        ));
    }
}
