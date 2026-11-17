//! Cross-page scenarios. Run in an isolated child so App startup cannot touch user data.
#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::components::{
    system_settings::SystemTab,
    system_settings_apps::catalog::CatalogRow,
    system_settings_search::{self, SystemSettingsSearchTarget},
};
use iced::advanced::{clipboard, renderer::{Headless, Style}};
use iced::{Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_api_app_identity::{AppCatalogEntry, AppId, AppReadResponse};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_THIRD_PARTY_08_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = config_ui::GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    ).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    false
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.apps_catalog.host = "隔离主机".into();
    app.apps_catalog.loading = false;
    app.apps_catalog.rows = [AppId::Codex, AppId::OpenClaw, AppId::Hermes].into_iter()
        .map(|id| CatalogRow {
            entry: AppCatalogEntry {
                id, name: format!("中文应用 {id:?}"), cli_command: None, runtime_kinds: vec![],
            },
            observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
        }).collect();
    app
}

#[test]
fn third_party_task_08_scope_and_navigation() {
    if !isolated("third_party_task_08_scope_and_navigation") { return; }
    let mut app = fixture();
    let locale = app.general_settings.desktop_locale;
    for id in [AppId::Codex, AppId::OpenClaw, AppId::Hermes] {
        app.apps_catalog.expanded = Some(id);
        for &(page, _) in ThirdPartyPage::ALL {
            let previous = app.apps_catalog.navigation_generation;
            let _ = navigate(&mut app, page);
            assert_eq!(app.apps_catalog.third_party_page, page);
            assert_eq!(app.apps_catalog.expanded, Some(id));
            assert_eq!(app.apps_catalog.host, "隔离主机");
            assert_ne!(previous, app.apps_catalog.navigation_generation);
            assert_eq!(app.general_settings.desktop_locale, locale);
        }
    }
    app.apps_catalog.connection_open = true;
    invalidate(&mut app);
    assert!(!app.apps_catalog.connection_open);
    assert!(app.apps_catalog.third_party_mcp_scope.is_none());
    assert!(app.app_providers.scope.is_none());
    assert!(app.agent_apps_auth.form.is_none());
}

#[test]
fn third_party_task_08_search_keeps_native_and_third_party_targets_separate() {
    if !isolated("third_party_task_08_search_keeps_native_and_third_party_targets_separate") { return; }
    let app = fixture();
    let entries = system_settings_search::catalog(&app);
    for &(page, _) in ThirdPartyPage::ALL {
        let matches: Vec<_> = entries.iter()
            .filter(|entry| entry.target == SystemSettingsSearchTarget::ThirdParty(page)).collect();
        assert_eq!(matches.len(), 1, "{page:?}");
        assert!(matches[0].matches("cc-switch"));
        assert_eq!(matches[0].target.page(), SystemTab::Apps);
    }
    assert!(entries.iter().any(|entry| matches!(entry.target, SystemSettingsSearchTarget::AgentMcp(_))));
    assert!(entries.iter().any(|entry| matches!(entry.target, SystemSettingsSearchTarget::AgentSkills(_))));
}

/// Shapes and rasterizes dynamic Chinese text; this is not native-window acceptance.
#[tokio::test(flavor = "multi_thread")]
async fn third_party_task_08_pages_rasterize_in_both_themes() {
    if !isolated("third_party_task_08_pages_rasterize_in_both_themes") { return; }
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() { fonts.load_font(font); }
    }
    let mut renderer = Renderer::new(
        Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"),
    ).await.unwrap();
    let mut app = fixture();
    for theme in [Theme::Light, Theme::Dark] {
        for size in [Size::new(800_u32, 720_u32), Size::new(1280, 900)] {
            for &(page, _) in ThirdPartyPage::ALL {
                app.apps_catalog.third_party_page = page;
                for selected in [None, Some(AppId::OpenClaw)] {
                    app.apps_catalog.expanded = selected;
                    app.apps_catalog.error = Some("隔离场景：读取失败，请重试".into());
                    let mut ui = UserInterface::build(
                        overlays(&app, view(&app)),
                        Size::new(size.width as f32, size.height as f32),
                        Cache::new(), &mut renderer,
                    );
                    let mut messages = Vec::new();
                    ui.update(&[], mouse::Cursor::Unavailable, &mut renderer, &mut clipboard::Null, &mut messages);
                    ui.draw(&mut renderer, &theme, &Style { text_color: theme.palette().text }, mouse::Cursor::Unavailable);
                    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                    assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4, "{page:?}");
                }
            }
        }
    }
}

#[test]
fn third_party_task_08_previous_context_response_cannot_replace_current_catalog() {
    if !isolated("third_party_task_08_previous_context_response_cannot_replace_current_catalog") { return; }
    let mut app = fixture();
    let old_context = crate::app::state::AgentAppsContext {
        gateway: "http://isolated.invalid".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let old_generation = crate::app::state::agent_apps::begin(&mut app.apps_catalog, old_context.clone());
    let mut current = old_context.clone();
    current.project = Some("/隔离项目/新作用域".into());
    let generation = crate::app::state::agent_apps::begin(&mut app.apps_catalog, current.clone());
    assert!(!crate::app::state::agent_apps::finish(
        &mut app.apps_catalog, &old_context, old_generation, Ok(vec![]),
    ));
    assert_eq!(app.apps_catalog.context.as_ref(), Some(&current));
    assert_eq!(app.apps_catalog.generation, generation);
    assert!(app.apps_catalog.loading);
}

#[test]
fn third_party_task_08_previous_host_observation_is_ignored() {
    if !isolated("third_party_task_08_previous_host_observation_is_ignored") { return; }
    let mut app = fixture();
    let context = crate::app::state::AgentAppsContext {
        gateway: "http://isolated.invalid".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let state = &mut app.apps_catalog;
    state.context = Some(context.clone());
    state.loading = true;
    let generation = state.generation;
    let (accepted, error) = desktop_settings_apps::system_settings_apps::catalog_update::finish_observation(
        desktop_settings_apps::system_settings_apps::catalog_update::ObservationInput {
            context: &state.context, host: &state.host, generation,
            loading: &mut state.loading, rows: &mut state.rows,
        },
        Some(&context), context.clone(), "旧主机".into(), generation, AppId::Codex,
        Ok(AppReadResponse { installations: vec![], desktop: None }),
    );
    assert!(!accepted);
    assert!(error.is_none());
    assert!(state.loading);
    assert_eq!(state.host, "隔离主机");
    assert_eq!(state.rows.len(), 3);
}
