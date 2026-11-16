//! Host contracts: isolated gateway resources and App-to-domain rendering projection.
use crate::app::components::system_settings_general;
use crate::app::message::SettingsMessage;
use crate::app::state::GeneralSettingsTab;
use crate::app::{App, Message};
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use iced::advanced::renderer::Headless;
use iced::{Element, Font, Renderer, Theme};
#[path = "../../../../../libs/desktop-settings-shell/src/host_module_30_tests/tab_appearance.rs"]
mod shared;
pub(in crate::app) type TabAppearance<'a> = shared::TabAppearance<'a, Message>;
impl shared::AppearanceMessage for Message {
    type Settings = SettingsMessage;
    fn none() -> Self {
        Self::None
    }
    fn settings(self) -> SettingsMessage {
        match self {
            Self::Settings(message) => message,
            message => panic!("expected settings message, got {message:?}"),
        }
    }
}
impl shared::AppearanceContext<Message> for App {
    fn locale(&self) -> DesktopLocale {
        self.general_settings.desktop_locale
    }
    fn general_view(&self) -> Element<'_, Message> {
        system_settings_general::view(self)
    }
}
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_CORE_TABS_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let test = test.split_once("::").unwrap().1;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

pub(in crate::app::components) async fn run(
    test: &str,
    fixture: impl Fn() -> App,
    check: impl Fn(&mut TabAppearance<'_>, &mut App),
) {
    run_at_least(test, 320, fixture, check).await;
}

pub(in crate::app) async fn run_at_least(
    test: &str,
    min_width: u32,
    fixture: impl Fn() -> App,
    check: impl Fn(&mut TabAppearance<'_>, &mut App),
) {
    if !isolated(test) {
        return;
    }
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut app = fixture();
            app.general_settings.desktop_locale = locale;
            app.appearance.app_theme = theme.clone();
            app.general_settings.general_settings_tab = GeneralSettingsTab::Interface;
            let width = if locale == DesktopLocale::English { min_width } else { 640 };
            let mut scene = TabAppearance {
                renderer: &mut renderer,
                theme,
                width,
                message: std::marker::PhantomData,
            };
            check(&mut scene, &mut app);
        }
    }
}
