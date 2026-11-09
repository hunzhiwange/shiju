use super::preferences::*;
use vw_gateway_client::vw_api_types::agent_apps::*;
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
fn cc_apps_desktop_preferences_state_only_api_results_commit() {
    let original = value();
    let mut state = PreferencesState {
        saved: Some(original.clone()),
        draft: Some(original.clone()),
        busy: true,
        generation: 2,
        ..Default::default()
    };
    state.draft.as_mut().unwrap().host.preferred_terminal = Some("unsaved".into());
    assert!(state.finish(2, Err("明确拒绝".into())));
    assert_eq!(state.saved.as_ref(), Some(&original));
    assert_eq!(state.draft.as_ref(), Some(&original));
    assert!(!state.busy);
    assert_eq!(state.error.as_deref(), Some("明确拒绝"));
    state.busy = true;
    let mut normalized = original.clone();
    normalized.host.preferred_terminal = Some("terminal".into());
    normalized.restart_required = true;
    assert!(state.finish(2, Ok(normalized.clone())));
    assert_eq!(state.saved.as_ref(), Some(&normalized));
    assert_eq!(state.draft.as_ref(), Some(&normalized));
    assert!(!state.finish(1, Ok(original)));
    assert_eq!(state.saved.as_ref(), Some(&normalized));
}
#[test]
fn cc_apps_desktop_preferences_state_last_visible_and_busy_reject_edits() {
    let mut state = PreferencesState { draft: Some(value()), ..Default::default() };
    state.visible(AppId::Codex, false);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Codex]);
    assert!(state.error.as_deref().unwrap().contains("至少"));
    state.visible(AppId::Claude, true);
    state.visible(AppId::Codex, false);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Claude]);
    state.busy = true;
    state.visible(AppId::Codex, true);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Claude]);
}
#[test]
fn cc_apps_desktop_preferences_state_host_capabilities_and_invalidation() {
    let mut state = PreferencesState { draft: Some(value()), ..Default::default() };
    assert!(!state.host_editable());
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    assert!(state.draft.as_ref().unwrap().host.wsl_shells.is_empty());
    state.draft.as_mut().unwrap().capabilities = Some(AppPreferencesCapabilities {
        terminals: vec!["terminal".into()],
        wsl_preferences: false,
    });
    assert!(state.host_editable());
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    assert!(state.draft.as_ref().unwrap().host.wsl_shells.is_empty());
    state.draft.as_mut().unwrap().capabilities.as_mut().unwrap().wsl_preferences = true;
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    state.wsl_edit(AppId::Codex, "-lc".into(), true);
    assert_eq!(
        state.draft.as_ref().unwrap().host.wsl_shells,
        vec![AppWslPreference {
            app_id: AppId::Codex,
            shell: Some("bash".into()),
            flag: Some("-lc".into())
        }]
    );
    state.wsl_edit(AppId::ClaudeDesktop, "bash".into(), false);
    assert_eq!(state.draft.as_ref().unwrap().host.wsl_shells.len(), 1);
    state.busy = true;
    let generation = state.generation;
    state.invalidate();
    assert!(!state.finish(generation, Ok(value())));
    assert!(state.saved.is_none() && state.draft.is_none() && !state.open);
}

#[test]
fn cc_apps_desktop_preferences_state_message_scopes_and_atomic_domains() {
    use crate::app::message::{SettingsMessage, settings::AppsCatalogMessage};
    let mut app = crate::app::App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.context = Some(context.clone());
    app.apps_catalog.host = "中文主机".into();
    app.apps_catalog.token = "isolated-fixture-application-token".into();
    app.apps_catalog.preferences.saved = Some(value());
    app.apps_catalog.preferences.draft = Some(value());
    let route = |app: &mut crate::app::App, value| {
        crate::app::message::settings::update(
            app,
            SettingsMessage::AppsCatalog(AppsCatalogMessage::Preferences(value)),
        )
    };
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    assert!(app.apps_catalog.preferences.busy);
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let generation = app.apps_catalog.preferences.generation;
    let mut normalized = value();
    normalized.host.preferred_terminal = Some("server-normalized".into());
    let reply = |host: &str, catalog_generation, result| PreferencesMessage::Reply {
        context: context.clone(),
        host: host.into(),
        catalog_generation,
        generation,
        result,
    };
    let cg = app.apps_catalog.generation;
    let _ = route(&mut app, reply("wrong-host", cg, Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let cg = app.apps_catalog.generation;
    let _ = route(&mut app, reply("中文主机", cg.wrapping_add(1), Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let _ = route(&mut app, reply("中文主机", cg, Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(normalized));
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::AppsCatalog(AppsCatalogMessage::HostInput("other".into())),
    );
    assert!(app.apps_catalog.preferences.saved.is_none());
    let _ = route(&mut app, reply("中文主机", cg, Ok(value())));
    assert!(app.apps_catalog.preferences.saved.is_none());
}

#[test]
fn cc_apps_desktop_preferences_state_list_uses_saved_order_and_visibility() {
    use super::catalog::CatalogRow;
    use crate::app::state::AppsCatalogState;
    let mut state = AppsCatalogState::default();
    state.rows = [AppId::Claude, AppId::Codex, AppId::Gemini]
        .into_iter()
        .map(|id| CatalogRow {
            entry: AppCatalogEntry {
                id,
                name: format!("{id:?}"),
                cli_command: None,
                runtime_kinds: vec![],
            },
            observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
        })
        .collect();
    let mut saved = value();
    saved.user.app_order = vec!["codex".into(), "claude".into(), "gemini".into()];
    saved.user.visible_apps = vec![AppId::Codex, AppId::Claude];
    state.preferences.saved = Some(saved.clone());
    state.preferences.draft = Some(value());
    assert_eq!(
        visible_rows(&state).iter().map(|r| r.entry.id).collect::<Vec<_>>(),
        vec![AppId::Codex, AppId::Claude]
    );
    saved.user.visible_apps = vec![AppId::Gemini];
    state.preferences.busy = true;
    assert!(state.preferences.finish(0, Ok(saved)));
    assert_eq!(
        visible_rows(&state).iter().map(|r| r.entry.id).collect::<Vec<_>>(),
        vec![AppId::Gemini]
    );
}

use crate::app::i18n::I18n;
use crate::app::{Message, components::system_settings_apps};
use iced::advanced::clipboard;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::{Id, Operation};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

pub(crate) const HELP_LOCALES: [(DesktopLocale, Theme, u32); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
    (DesktopLocale::English, Theme::Light, 640),
    (DesktopLocale::TraditionalChinese, Theme::Light, 420),
];

#[derive(Default)]
struct Labels(Vec<(String, Rectangle)>);

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}

pub(crate) async fn verify_help(
    view: Element<'_, Message>,
    name: &str,
    mode: &(DesktopLocale, Theme, u32),
    help: &[(&str, &str)],
    visible: &[&str],
    action: &str,
) -> Vec<Message> {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let (locale, theme, width) = mode;
    let size = Size::new(*width, 650);
    let mut ui = UserInterface::build(
        view,
        Size::from((size.width, size.height)),
        Cache::new(),
        &mut renderer,
    );
    let i18n = I18n::new(*locale);
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    let bounds = |label| {
        labels
            .0
            .iter()
            .rfind(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("{name}: missing label {label:?}"))
            .1
    };
    let visible_label = |source| {
        if labels.0.iter().any(|(label, _)| label == source) { source } else { i18n.__(source) }
    };
    for label in visible {
        bounds(visible_label(label));
    }

    let mut messages = Vec::new();
    let mut send = |ui: &mut UserInterface<'_, Message, Theme, Renderer>,
                    renderer: &mut Renderer,
                    cursor,
                    events: &[Event]| {
        ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    };
    let snapshot = |ui: &mut UserInterface<'_, Message, Theme, Renderer>,
                    renderer: &mut Renderer,
                    cursor,
                    frame: &str| {
        ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
            image::save_buffer(
                std::path::PathBuf::from(directory).join(format!("{name}-{locale:?}-{frame}.png")),
                &pixels,
                size.width,
                size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        pixels
    };
    let idle = snapshot(&mut ui, &mut renderer, mouse::Cursor::Unavailable, "idle");
    for (index, (title, description)) in help.iter().enumerate() {
        assert!(
            labels.0.iter().all(|(label, _)| label != description && label != i18n.__(description)),
            "{name}: optional explanation {description:?} must be hidden while idle"
        );
        let title_bounds = bounds(i18n.__(title));
        let point = Point::new(title_bounds.x + title_bounds.width + 15.0, title_bounds.center_y());
        assert!(Rectangle::with_size(Size::from((size.width, size.height))).contains(point));
        let cursor = mouse::Cursor::Available(point);
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
        );
        let opened = snapshot(&mut ui, &mut renderer, cursor, &format!("open-{index}"));
        assert!(
            opened != idle,
            "{name}: hovering over the question mark beside {title:?} must open help"
        );
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)],
        );
        let closed = snapshot(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &format!("closed-{index}"),
        );
        assert!(closed == idle, "{name}: leaving the question mark must close unpinned help");
    }

    let point = bounds(visible_label(action)).center();
    send(
        &mut ui,
        &mut renderer,
        mouse::Cursor::Available(point),
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    );
    messages
}

#[tokio::test]
async fn help_rollout_04_renders_contextual_help() {
    use crate::app::message::settings::{AppsCatalogMessage, SettingsMessage};
    use crate::app::state::AppsCatalogState;

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
