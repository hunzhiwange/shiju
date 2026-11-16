use desktop_theme::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, set_test_themes_root,
};
#[test]
fn app_theme_options_include_local_custom_themes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    store
        .save(&ThemeDefinition {
            id: "mint-night".to_string(),
            name: "Mint Night".to_string(),
            tagline: None,
            palette: ThemePaletteHex {
                background: "#0b1220".to_string(),
                text: "#e2e8f0".to_string(),
                primary: "#34d399".to_string(),
                success: "#22c55e".to_string(),
                warning: "#f59e0b".to_string(),
                danger: "#ef4444".to_string(),
            },
            assets: ThemeAssets::default(),
            chrome: ThemeChrome::default(),
        })
        .expect("save custom theme");
    set_test_themes_root(Some(temp.path().to_path_buf()));

    let themes = crate::general::app_theme_options(&iced::Theme::Light);
    assert!(themes.iter().any(|theme| theme.to_string() == "Mint Night"));

    set_test_themes_root(None);
}

use crate::general::{self, Action, View};
use desktop_settings_shell::GeneralSettingsTab;
type Message = Action<&'static str>;
fn test_app() -> View<'static, &'static str> {
    View {
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &iced::Theme::Light,
        shell: "sh",
        shell_options: vec!["sh"],
        font_family: "",
        displayed_font_size: 14.0,
        preview_auto_save_mode: config_ui::PreviewAutoSaveMode::Off,
        chat_timezone: "Asia/Shanghai",
        launch_at_login: false,
        launch_at_login_busy: false,
        close_to_tray: false,
        task_completion_notification: false,
        notification_supported: false,
        general_settings_tab: GeneralSettingsTab::Interface,
    }
}
fn view<'a>(app: &View<'a, &'static str>) -> iced::Element<'a, Message> {
    general::view(
        app,
        |action| action,
        |_, body, _, _| iced::widget::text(body).into(),
        iced::widget::Space::new().into(),
    )
}
#[test]
fn terminal_font_size_control_matches_the_other_terminal_fields() {
    iced::futures::executor::block_on(async {
        use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
        use iced::{Font, Pixels, Size};

        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia renderer");
        let mut app = test_app();
        app.general_settings_tab = GeneralSettingsTab::Terminal;

        let mut element = view(&app);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(900.0, 600.0));
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let terminal_tab = Layout::new(&node).children().nth(2).expect("terminal tab");
        let section = terminal_tab.children().nth(1).expect("terminal section");
        let fields = section.children().next().expect("terminal fields");

        for field in fields.children().step_by(2) {
            let row = field.children().next().expect("field row");
            let wrapper = row.children().nth(1).expect("control wrapper");
            let control = wrapper
                .children()
                .next()
                .expect("bounded control")
                .children()
                .next()
                .expect("terminal control");
            assert_eq!(control.bounds().width, 280.0);
        }
    });
}
