//! 验证通用设置的主题、终端、字体、自动保存与时区选项。
//! 选择 helper 必须原样保留当前配置，自定义主题目录则补入可选项。

use super::*;
use crate::app::state::GeneralSettingsTab;
use crate::app::theme_catalog::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, set_test_themes_root,
};
use crate::app::{App, PreviewAutoSaveMode, Shell, TerminalTheme, message};
use iced::widget::text;

// 通用设置测试从相同默认配置派生，避免平台默认值造成分支差异。
fn test_app() -> App {
    App::new().0
}

// 构建后立即消费元素，用于证明选项组合满足 Iced 的生命周期约束。
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn system_settings_general_tests_are_wired() {
    assert!(module_path!().contains("system_settings_general_tests"));
}

#[test]
fn setting_row_builds_text_and_control_columns() {
    keep_element(setting_row("标签", "设置说明", text("control")));
}

#[test]
fn selection_helpers_preserve_theme_shell_and_terminal_theme() {
    match app_theme_selected(iced::Theme::Dark) {
        Message::View(message::ViewMessage::AppThemeSelected(theme)) => {
            assert_eq!(theme, iced::Theme::Dark);
        }
        other => panic!("unexpected theme message: {other:?}"),
    }

    for shell in Shell::all() {
        match terminal_shell_selected(shell) {
            Message::Terminal(message::TerminalMessage::ShellSelected(selected)) => {
                assert_eq!(selected, shell);
            }
            other => panic!("unexpected shell message: {other:?}"),
        }
    }

    match terminal_theme_selected(TerminalTheme::System) {
        Message::Terminal(message::TerminalMessage::ThemeSelected(theme)) => {
            assert_eq!(theme, TerminalTheme::System);
        }
        other => panic!("unexpected terminal theme message: {other:?}"),
    }
}

#[test]
fn terminal_font_helper_preserves_empty_and_named_families() {
    for font_family in ["", "JetBrains Mono", "自定义字体"] {
        match terminal_font_selected(font_family.to_string()) {
            Message::Terminal(message::TerminalMessage::FontSelected(selected)) => {
                assert_eq!(selected, font_family);
            }
            other => panic!("unexpected terminal font message: {other:?}"),
        }
    }
}

#[test]
fn auto_save_helper_preserves_every_supported_mode() {
    for mode in PreviewAutoSaveMode::ALL {
        match preview_auto_save_mode_changed(mode) {
            Message::Preview(message::PreviewMessage::AutoSaveModeChanged(selected)) => {
                assert_eq!(selected, mode);
            }
            other => panic!("unexpected auto-save message: {other:?}"),
        }
    }
}

#[test]
fn timezone_helper_preserves_supported_iana_timezone() {
    match timezone_selected("America/New_York".to_string()) {
        Message::Settings(message::SettingsMessage::General(
            message::GeneralSettingsMessage::TimezoneSelected(timezone),
        )) => assert_eq!(timezone, "America/New_York"),
        other => panic!("unexpected timezone message: {other:?}"),
    }
}

#[test]
fn view_builds_each_general_settings_tab() {
    let mut app = test_app();

    for tab in [
        GeneralSettingsTab::Interface,
        GeneralSettingsTab::Terminal,
        GeneralSettingsTab::Editor,
        GeneralSettingsTab::Themes,
    ] {
        app.general_settings_tab = tab;
        keep_element(view(&app));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_font_size_control_matches_the_other_terminal_fields() {
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
        let control = wrapper.children().next().expect("terminal control");
        assert_eq!(control.bounds().width, TERMINAL_CONTROL_WIDTH);
    }
}

#[test]
fn font_size_helper_preserves_slider_boundaries_and_fraction() {
    for font_size in [10.0, 13.5, 18.0] {
        match terminal_font_size_changed(font_size) {
            Message::Terminal(message::TerminalMessage::FontSizePreview(selected)) => {
                assert_eq!(selected, font_size);
            }
            other => panic!("unexpected terminal font size message: {other:?}"),
        }
    }
}

#[test]
fn view_builds_supported_general_setting_combinations() {
    let mut app = test_app();
    keep_element(view(&app));

    app.app_theme = iced::Theme::Dark;
    for timezone in ["Asia/Shanghai", "America/New_York"] {
        app.chat_timezone = timezone.to_string();
        keep_element(view(&app));
    }
    for shell in Shell::all() {
        app.terminal.shell = shell;
        for (font_family, font_size, auto_save_mode) in [
            ("JetBrains Mono", 10.0, PreviewAutoSaveMode::Off),
            ("Fira Code", 13.5, PreviewAutoSaveMode::AfterDelay),
            ("Menlo", 16.0, PreviewAutoSaveMode::OnFocusChange),
            ("Monaco", 18.0, PreviewAutoSaveMode::OnWindowChange),
        ] {
            app.terminal.font_family = font_family.to_string();
            app.terminal.font_size = font_size;
            app.preview_auto_save_mode = auto_save_mode;
            keep_element(view(&app));
        }
    }
}

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

    let themes = app_theme_options(&test_app());
    assert!(themes.iter().any(|theme| theme.to_string() == "Mint Night"));

    set_test_themes_root(None);
}

#[tokio::test(flavor = "multi_thread")]
async fn general_settings_render_chinese_in_light_and_dark_without_overflow() {
    use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Rectangle, Size};

    let mut app = test_app();
    app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        app.app_theme = theme.clone();
        for (tab, tab_name) in [
            (GeneralSettingsTab::Interface, "interface"),
            (GeneralSettingsTab::Terminal, "terminal"),
        ] {
            app.general_settings_tab = tab;
            for width in [640, 960] {
                let mut renderer = <iced::Renderer as Headless>::new(
                    Font::DEFAULT,
                    Pixels(16.0),
                    Some("tiny-skia"),
                )
                .await
                .expect("tiny-skia renderer");
                let viewport = Rectangle::with_size(Size::new(width as f32, 1200.0));
                let mut element = view(&app);
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, viewport.size()),
                );
                fn assert_fits(layout: Layout<'_>, viewport: Rectangle) {
                    let bounds = layout.bounds();
                    assert!(
                        bounds.x >= 0.0 && bounds.x + bounds.width <= viewport.width + 0.5,
                        "horizontal overflow: {bounds:?}"
                    );
                    for child in layout.children() {
                        assert_fits(child, viewport);
                    }
                }
                assert_fits(Layout::new(&node), viewport);
                // 与真实窗口一样先派发重绘，初始化开关与选择框的交互状态。
                let mut messages = Vec::new();
                element.as_widget_mut().update(
                    &mut tree,
                    &iced::Event::Window(iced::window::Event::RedrawRequested(
                        iced::time::Instant::now(),
                    )),
                    Layout::new(&node),
                    iced::mouse::Cursor::Unavailable,
                    &renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut iced::advanced::Shell::new(&mut messages),
                    &viewport,
                );
                assert!(messages.is_empty(), "rendering must not change settings");
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &iced::advanced::renderer::Style { text_color: theme.palette().text },
                    Layout::new(&node),
                    iced::mouse::Cursor::Unavailable,
                    &viewport,
                );
                let pixels =
                    renderer.screenshot(Size::new(width, 1200), 1.0, theme.palette().background);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
                if let Ok(directory) = std::env::var("SHIJU_SETTINGS_PREVIEW_DIR") {
                    std::fs::create_dir_all(&directory).expect("preview directory");
                    image::save_buffer(
                        std::path::Path::new(&directory)
                            .join(format!("{tab_name}-{theme_name}-{width}.png")),
                        &pixels,
                        width,
                        1200,
                        image::ColorType::Rgba8,
                    )
                    .expect("save rendered settings");
                }
            }
        }
    }
}
