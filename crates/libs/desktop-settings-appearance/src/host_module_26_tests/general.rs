//! Domain page, help, event, and native rendering assertions without App.
use super::ui_fixture::{self, general_view as view, *};
use config_ui::PreviewAutoSaveMode;
use desktop_ui_controls::settings_navigation::{
    GeneralSettingsTab,
};
use desktop_terminal::shell::Shell;
use iced::widget::text;
use iced::{Element, Length, Theme};
fn test_app() -> Fixture {
    Fixture::default()
}
fn general_field_with_help<'a>(
    app: &Fixture,
    field: &GeneralSettingField,
    description: impl Into<std::borrow::Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    crate::headings::settings_flexible_row_with_help(
        help,
        i18n,
        &app.app_theme,
        i18n.__(field.label),
        description,
        280.0,
        control,
    )
    .id(field.id)
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
    let app = test_app();
    keep_element(
        general_field_with_help(&app, &general_fields::LANGUAGE, "设置说明", text("control"))
            .into(),
    );
}

#[test]
fn view_builds_each_general_settings_tab() {
    let mut app = test_app();

    for tab in [
        GeneralSettingsTab::Interface,
        GeneralSettingsTab::Terminal,
        GeneralSettingsTab::Editor,
        GeneralSettingsTab::Themes,
        GeneralSettingsTab::Companion,
    ] {
        app.general_settings_tab = tab;
        keep_element(view(&app));
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

#[tokio::test(flavor = "multi_thread")]
async fn general_settings_render_chinese_in_light_and_dark_without_overflow() {
    use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Rectangle, Size};

    let mut app = test_app();
    app.desktop_locale = config_ui::DesktopLocale::SimplifiedChinese;
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

#[cfg(not(target_arch = "wasm32"))]
mod tab_appearance {
    use super::*;
    use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
    use desktop_i18n::I18n;
    use iced::advanced::{
        Layout, clipboard, layout,
        renderer::{Headless, Style},
        widget::{Id, Operation, Tree},
    };
    use iced::{Color, Event, Font, Point, Rectangle, Renderer, Size, keyboard, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    const SIZE: Size<u32> = Size::new(960, 640);
    type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

    fn run_isolated(test: &str) -> bool {
        const CHILD: &str = "SHIJU_GENERAL_TABS_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            return true;
        }
        let home = tempfile::tempdir().expect("isolated settings directory");
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
        let module = module_path!().split_once("::").unwrap().1;
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test}")])
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
            String::from_utf8_lossy(&output.stderr),
        );
        false
    }

    #[derive(Default)]
    struct Labels(Vec<String>);

    impl Operation for Labels {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
            self.0.push(value.into());
        }
    }

    fn tab_bounds(element: &mut Element<'_, Message>, renderer: &Renderer) -> Vec<Rectangle> {
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::from((SIZE.width, SIZE.height)));
        let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
        let row = Layout::new(&node).children().nth(1).expect("tab row");
        row.children()
            .map(|button| {
                let bounds = button.bounds();
                let label = button.children().next().expect("tab label").bounds();
                let offset = if cfg!(target_os = "macos") { 0.5 } else { 0.0 };
                assert_eq!(label.x - bounds.x, 14.0);
                assert_eq!(label.center_y(), bounds.center_y() - offset);
                assert_eq!(bounds.height - label.height, 16.0);
                bounds
            })
            .collect()
    }

    fn page_help_bounds(element: &mut Element<'_, Message>, renderer: &Renderer) -> Rectangle {
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::from((SIZE.width, SIZE.height)));
        let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
        Layout::new(&node)
            .children()
            .next()
            .expect("page heading")
            .children()
            .nth(1)
            .expect("help icon")
            .bounds()
    }

    fn send(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        cursor: mouse::Cursor,
        event: Event,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        ui.update(&[event], cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    fn capture_at_size(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        size: Size<u32>,
    ) -> image::RgbaImage {
        assert!(
            send(
                ui,
                renderer,
                cursor,
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            )
            .is_empty()
        );
        ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
        image::RgbaImage::from_raw(size.width, size.height, pixels).expect("screenshot")
    }

    fn capture(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
    ) -> image::RgbaImage {
        capture_at_size(ui, renderer, theme, cursor, SIZE)
    }

    fn save_tabs(pixels: &image::RgbaImage, tabs: &[Rectangle], name: &str) {
        if let Some(directory) = std::env::var_os("SHIJU_GENERAL_TABS_SCREENSHOTS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("screenshot directory");
            let first = tabs[0];
            let last = tabs[3];
            image::imageops::crop_imm(
                pixels,
                first.x as u32,
                first.y as u32,
                (last.x + last.width - first.x).ceil() as u32,
                first.height.ceil() as u32,
            )
            .to_image()
            .save(directory.join(format!("{name}.png")))
            .expect("save tab screenshot");
        }
    }

    fn assert_color(pixel: &image::Rgba<u8>, expected: Color) {
        for (actual, expected) in pixel.0.into_iter().zip(expected.into_rgba8()) {
            assert!(actual.abs_diff(expected) <= 2, "pixel {pixel:?}, expected {expected}");
        }
    }

    fn fill_pixel(pixels: &image::RgbaImage, bounds: Rectangle) -> &image::Rgba<u8> {
        pixels.get_pixel(bounds.x as u32 + 5, bounds.center_y() as u32)
    }

    fn assert_hover_help(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        theme: &Theme,
        idle: &image::RgbaImage,
        icon: Rectangle,
        title: &str,
        description: &str,
    ) {
        let mut hover = |ui: &mut Ui<'_>, point| {
            let cursor = mouse::Cursor::Available(point);
            assert!(
                send(
                    ui,
                    renderer,
                    cursor,
                    Event::Mouse(mouse::Event::CursorMoved { position: point })
                )
                .is_empty()
            );
        };
        let trigger = icon.center();
        let gap = Point::new(icon.x + icon.width + 2.0, trigger.y);
        let card = Point::new(gap.x + 20.0, trigger.y);
        hover(ui, trigger);
        hover(ui, gap);
        hover(ui, card);
        let cursor = mouse::Cursor::Available(card);
        let open = capture(ui, renderer, theme, cursor);
        assert!(open != *idle, "hover help is drawn");
        let outside = Point::new(icon.x, 0.0);
        assert!(
            send(
                ui,
                renderer,
                mouse::Cursor::Available(outside),
                Event::Mouse(mouse::Event::CursorMoved { position: outside }),
            )
            .is_empty()
        );
        assert!(
            capture(ui, renderer, theme, mouse::Cursor::Unavailable) == *idle,
            "help closes on pointer exit"
        );

        let left = mouse::Button::Left;
        assert!(
            send(
                ui,
                renderer,
                mouse::Cursor::Available(trigger),
                Event::Mouse(mouse::Event::ButtonPressed(left))
            )
            .is_empty()
        );
        assert!(
            send(
                ui,
                renderer,
                mouse::Cursor::Available(outside),
                Event::Mouse(mouse::Event::CursorMoved { position: outside })
            )
            .is_empty()
        );
        let pinned = capture(ui, renderer, theme, mouse::Cursor::Available(outside));
        assert!(pinned != *idle, "left click pins help");

        let start = Point::new(icon.x + icon.width + 4.0 + 12.5, icon.y + 16.0);
        let end = Point::new(start.x + 250.0, start.y + 150.0);
        for (point, event) in [
            (start, mouse::Event::ButtonPressed(left)),
            (end, mouse::Event::CursorMoved { position: end }),
            (end, mouse::Event::ButtonReleased(left)),
        ] {
            assert!(
                send(ui, renderer, mouse::Cursor::Available(point), Event::Mouse(event)).is_empty()
            );
        }
        let selected = capture(ui, renderer, theme, mouse::Cursor::Available(outside));
        assert!(selected != pinned, "drag highlights the text");
        for (point, event) in [
            (outside, mouse::Event::CursorMoved { position: outside }),
            (start, mouse::Event::CursorMoved { position: start }),
            (
                start,
                mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1.0 },
                },
            ),
        ] {
            assert!(
                send(ui, renderer, mouse::Cursor::Available(point), Event::Mouse(event)).is_empty()
            );
        }
        assert!(
            capture(ui, renderer, theme, mouse::Cursor::Available(outside)) == selected,
            "moving and incidental scrolling preserve the selection"
        );
        let expected = format!("{title}\n{description}");
        let copy_key = keyboard::Key::Character("c".into());
        let shortcut = Event::Keyboard(keyboard::Event::KeyPressed {
            key: copy_key.clone(),
            modified_key: copy_key,
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
            location: keyboard::Location::Standard,
            modifiers: if cfg!(target_os = "macos") {
                keyboard::Modifiers::LOGO
            } else {
                keyboard::Modifiers::CTRL
            },
            text: None,
            repeat: false,
        });
        assert!(matches!(
            send(ui, renderer, mouse::Cursor::Available(outside), shortcut).as_slice(),
            [Message::CopyCode(help)] if help == &expected
        ));

        let query = urlencoding::encode(&expected);
        let actions = [
            Message::CopyCode(expected.clone()),
            Message::AppendText(expected.clone()),
            Message::OpenUrl(format!("https://www.baidu.com/s?wd={query}")),
            Message::OpenUrl(format!("https://www.google.com/search?q={query}")),
            Message::OpenUrl(format!("https://www.bing.com/search?q={query}")),
        ];
        let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
        let row_height = f32::from(line_height) + 10.0;
        for (index, action) in actions.into_iter().enumerate() {
            let cursor = mouse::Cursor::Available(start);
            assert!(
                send(
                    ui,
                    renderer,
                    cursor,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                )
                .is_empty(),
                "right click opens a menu without running an action"
            );
            let menu_idle = capture(ui, renderer, theme, cursor);
            assert!(menu_idle != selected, "menu is visible");
            let item =
                Point::new(start.x + 24.0, start.y + 3.0 + row_height * (index as f32 + 0.5));
            let item_cursor = mouse::Cursor::Available(item);
            assert!(
                send(
                    ui,
                    renderer,
                    item_cursor,
                    Event::Mouse(mouse::Event::CursorMoved { position: item })
                )
                .is_empty()
            );
            assert!(
                capture(ui, renderer, theme, item_cursor) != menu_idle,
                "menu item is highlighted"
            );
            assert!(
                send(ui, renderer, item_cursor, Event::Mouse(mouse::Event::ButtonPressed(left)))
                    .is_empty()
            );
            let messages =
                send(ui, renderer, item_cursor, Event::Mouse(mouse::Event::ButtonReleased(left)));
            assert!(
                matches!(
                    (messages.as_slice(), &action),
                    ([Message::CopyCode(actual)], Message::CopyCode(expected))
                    | ([Message::AppendText(actual)], Message::AppendText(expected))
                    | ([Message::OpenUrl(actual)], Message::OpenUrl(expected))
                        if actual == expected
                ),
                "{messages:?} did not match {action:?}"
            );
            assert!(
                capture(ui, renderer, theme, mouse::Cursor::Available(outside)) == selected,
                "the menu closes and the selection stays visible after choosing an action"
            );
        }
        assert!(
            send(
                ui,
                renderer,
                mouse::Cursor::Available(outside),
                Event::Mouse(mouse::Event::ButtonPressed(left))
            )
            .is_empty()
        );
        assert!(
            capture(ui, renderer, theme, mouse::Cursor::Unavailable) == *idle,
            "outside click closes pinned help"
        );
    }

    #[derive(Default)]
    struct TextBounds(Vec<(String, Rectangle)>);

    impl Operation for TextBounds {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
            self.0.push((value.into(), bounds));
        }
    }

    async fn check_contextual_help<'a>(
        app: &Fixture,
        view: impl Fn() -> Element<'a, Message>,
        name: &str,
        title: &str,
        description: &str,
        visible: &str,
    ) {
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
                .await
                .expect("tiny-skia renderer");
        let i18n = I18n::new(app.desktop_locale);
        let size = Size::new(520, 780);
        let mut ui = UserInterface::build(
            iced::widget::responsive(|_| view()),
            Size::from((size.width, size.height)),
            Cache::new(),
            &mut renderer,
        );
        let idle = capture_at_size(
            &mut ui,
            &mut renderer,
            &app.app_theme,
            mouse::Cursor::Unavailable,
            size,
        );
        // Redraw resolves responsive modal layout before locating the help icon.
        let mut labels = TextBounds::default();
        ui.operate(&renderer, &mut labels);
        let title_bounds = labels.0.iter().rfind(|(label, _)| label == i18n.__(title)).unwrap().1;
        let point = Point::new(title_bounds.x + title_bounds.width + 15.0, title_bounds.center_y());
        let cursor = mouse::Cursor::Available(point);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                cursor,
                Event::Mouse(mouse::Event::CursorMoved { position: point })
            )
            .is_empty()
        );
        let opened = capture_at_size(&mut ui, &mut renderer, &app.app_theme, cursor, size);
        if let Some(directory) = std::env::var_os("SHIJU_HELP_ROLLOUT_03_SCREENSHOTS") {
            let directory = std::path::PathBuf::from(directory);
            idle.save(directory.join(format!("{name}-idle.png"))).unwrap();
            opened.save(directory.join(format!("{name}-open.png"))).unwrap();
        }
        assert!(idle != opened, "{name}: hovering beside {title:?} must draw the help card");
        assert!(
            labels.0.iter().all(|(label, _)| label != i18n.__(description)),
            "{name}: optional explanation is visible while idle"
        );
        assert!(
            labels.0.iter().any(|(label, _)| label == i18n.__(visible)),
            "{name}: {visible:?} must remain visible"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_03_terminal_renders_contextual_help() {
        if !run_isolated("help_rollout_03_terminal_renders_contextual_help") {
            return;
        }
        let mut app = test_app();
        app.general_settings_tab = GeneralSettingsTab::Terminal;
        app.terminal.font_size = 14.0;
        for (locale, theme, name) in [
            (DesktopLocale::English, Theme::Light, "terminal-en-light"),
            (DesktopLocale::SimplifiedChinese, Theme::Dark, "terminal-zh-dark"),
        ] {
            app.desktop_locale = locale;
            app.app_theme = theme;
            check_contextual_help(
                &app,
                || view(&app),
                name,
                "字体大小",
                "在 10pt 到 18pt 之间调整终端字号。",
                "14 pt",
            )
            .await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_03_preview_renders_contextual_help() {
        if !run_isolated("help_rollout_03_preview_renders_contextual_help") {
            return;
        }
        let mut app = test_app();
        for (locale, theme, name) in [
            (DesktopLocale::English, Theme::Light, "preview-en-light"),
            (DesktopLocale::SimplifiedChinese, Theme::Dark, "preview-zh-dark"),
        ] {
            app.desktop_locale = locale;
            app.app_theme = theme;
            check_contextual_help(
                &app,
                || {
                    ui_fixture::preview_overlay(
                        &app,
                        iced::widget::container(text("Preview"))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into(),
                    )
                },
                name,
                "编辑器设置",
                "统一调整字号、行高、语言与主题。",
                "完成",
            )
            .await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_03_editor_renders_contextual_help() {
        use super::super::ui_fixture as system_settings_editor;

        if !run_isolated("help_rollout_03_editor_renders_contextual_help") {
            return;
        }
        let mut app = test_app();
        app.desktop_locale = DesktopLocale::English;
        app.app_theme = Theme::Light;
        check_contextual_help(
            &app,
            || system_settings_editor::view(&app),
            "editor-standalone-en-light",
            "编辑器配置",
            "配置主题、字号与行高等编辑体验参数。",
            "主题与显示",
        )
        .await;

        app.desktop_locale = DesktopLocale::SimplifiedChinese;
        app.app_theme = Theme::Dark;
        app.general_settings_tab = GeneralSettingsTab::Editor;
        check_contextual_help(
            &app,
            || view(&app),
            "editor-tab-zh-dark",
            "主题与显示",
            "统一编辑器主题来源及排版参数。",
            "需要对应语言服务器可用。",
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_03_themes_render_contextual_help() {
        use super::super::ui_fixture::{editor, gallery};
        use crate::system_settings_themes::state::{ThemeEditorTab, ThemesSettingsMode};

        if !run_isolated("help_rollout_03_themes_render_contextual_help") {
            return;
        }
        let mut app = test_app();
        app.desktop_locale = DesktopLocale::English;
        app.app_theme = Theme::Light;
        app.themes_settings.entries = vec![gallery::ThemeGalleryEntry {
            id: "fixture-theme".into(),
            name: "Fixture theme".into(),
            tagline: Some("Fixture swatches".into()),
            source: gallery::ThemeGallerySource::Custom,
            palette: gallery::palette_hex_from_theme(&Theme::Dark),
            cover_path: None,
            modified: false,
        }];
        check_contextual_help(
            &app,
            || system_settings_themes::view(&app),
            "theme-gallery-en-light",
            "主题",
            "浏览内置、预设与自定义主题，应用或管理本地主题包。",
            "Fixture theme",
        )
        .await;

        app.desktop_locale = DesktopLocale::SimplifiedChinese;
        app.app_theme = Theme::Dark;
        app.general_settings_tab = GeneralSettingsTab::Themes;
        app.themes_settings.mode = ThemesSettingsMode::Edit;
        app.themes_settings.draft = Some(editor::new_theme_draft(&[]));
        app.themes_settings.error = Some("Fixture: invalid color".into());

        for (tab, name, title, description, visible) in [
            (
                ThemeEditorTab::Basics,
                "theme-basics-zh-dark",
                "名称",
                "显示在画廊与主题选择中。",
                "名称必填。",
            ),
            (
                ThemeEditorTab::Palette,
                "theme-palette-zh-dark",
                "背景",
                "面板与页面背景色。",
                "仅支持 #rrggbb 十六进制颜色。",
            ),
            (
                ThemeEditorTab::Assets,
                "theme-assets-zh-dark",
                "窗口背景",
                "主窗口壁纸；默认透明度供分区未单独设置时回退。",
                "画廊卡片缩略图，支持 png/jpg/jpeg/webp。",
            ),
            (
                ThemeEditorTab::Chrome,
                "theme-chrome-zh-dark",
                "输入背景",
                "可选；留空使用默认。",
                "Fixture: invalid color",
            ),
            (
                ThemeEditorTab::Preview,
                "theme-preview-zh-dark",
                "预览",
                "暗黑草稿下请确保文字可读。",
                "取消",
            ),
        ] {
            app.themes_settings.editor_tab = tab;
            check_contextual_help(
                &app,
                || {
                    crate::system_settings_themes::editor_overlay(
                        &inputs(&app),
                        // The host settings page fills the window. A standalone gallery
                        // shrinks the modal stack and clips the form's help targets.
                        iced::widget::container(gallery::view(&app))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into(),
                    )
                },
                name,
                title,
                description,
                visible,
            )
            .await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn settings_tabs_general_appearance() {
        if !run_isolated("settings_tabs_general_appearance") {
            return;
        }
        // Domain fixture has no gateway client; rendering is entirely local.
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
                .await
                .expect("tiny-skia renderer");
        let mut app = test_app();
        let tabs = [
            (GeneralSettingsTab::Interface, "开机启动"),
            (GeneralSettingsTab::Terminal, "终端 Shell"),
            (GeneralSettingsTab::Editor, "主题与显示"),
            (GeneralSettingsTab::Themes, "主题画廊"),
            (GeneralSettingsTab::Companion, "我的智能伙伴"),
        ];

        for (locale, locale_name, names) in [
            (
                DesktopLocale::SimplifiedChinese,
                "zh-CN",
                ["界面", "终端", "编辑器", "主题", "迷你界面和智能伙伴"],
            ),
            (
                DesktopLocale::TraditionalChinese,
                "zh-TW",
                ["介面", "終端機", "編輯器", "主題", "迷你介面和智慧夥伴"],
            ),
            (
                DesktopLocale::English,
                "en",
                ["Interface", "Terminal", "Editor", "Themes", "Mini and companions"],
            ),
        ] {
            app.desktop_locale = locale;
            let i18n = I18n::new(locale);
            for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                app.app_theme = theme.clone();
                app.general_settings_tab = GeneralSettingsTab::Interface;
                let background = theme.palette().background;
                let palette = theme.extended_palette();
                let strong = palette.background.strong.color;
                let neutral = Color::from_rgba(
                    (strong.r + background.r) / 2.0,
                    (strong.g + background.g) / 2.0,
                    (strong.b + background.b) / 2.0,
                    1.0,
                );

                for (index, (active, content)) in tabs.into_iter().enumerate() {
                    assert_eq!(app.general_settings_tab, active);
                    let (bounds, help) = {
                        let mut element = view(&app);
                        (
                            tab_bounds(&mut element, &renderer),
                            page_help_bounds(&mut element, &renderer),
                        )
                    };
                    assert_eq!(bounds.len(), names.len());
                    for pair in bounds.windows(2) {
                        assert!((pair[1].x - pair[0].x - pair[0].width - 8.0).abs() < 0.01);
                        assert_eq!(pair[0].center_y(), pair[1].center_y());
                    }
                    let mut ui = UserInterface::build(
                        iced::widget::responsive(|_| view(&app)),
                        Size::from((SIZE.width, SIZE.height)),
                        Cache::new(),
                        &mut renderer,
                    );
                    let mut labels = Labels::default();
                    ui.operate(&renderer, &mut labels);
                    assert_eq!(&labels.0[1..1 + names.len()], &names);
                    assert!(
                        labels.0[1 + names.len()..].iter().any(|label| label == &i18n.__(content)),
                        "{content:?} missing from {active:?}"
                    );

                    let idle = capture(&mut ui, &mut renderer, &theme, mouse::Cursor::Unavailable);
                    if active == GeneralSettingsTab::Interface {
                        let description = i18n.__("通过标签切换界面、终端、编辑器与主题配置。");
                        assert!(!labels.0.iter().any(|label| label == description));
                        assert!(
                            !labels.0.iter().any(|label| label == i18n.__("应用外观与预览行为。"))
                        );
                        assert_hover_help(
                            &mut ui,
                            &mut renderer,
                            &theme,
                            &idle,
                            help,
                            i18n.__("常规"),
                            description,
                        );
                    }
                    for (tab_index, bounds) in bounds.iter().copied().enumerate() {
                        assert_color(
                            fill_pixel(&idle, bounds),
                            if tab_index == index { neutral } else { background },
                        );
                    }
                    let selected = bounds[index];
                    assert_color(
                        idle.get_pixel(selected.x as u32 + 1, selected.y as u32 + 1),
                        background,
                    );
                    save_tabs(&idle, &bounds, &format!("{locale_name}-{theme_name}-{index}-idle"));

                    let Some(&(next, _)) = tabs.get(index + 1) else {
                        continue;
                    };
                    let target = bounds[index + 1];
                    let point = Point::new(target.center_x(), target.center_y());
                    let cursor = mouse::Cursor::Available(point);
                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            cursor,
                            Event::Mouse(mouse::Event::CursorMoved { position: point })
                        )
                        .is_empty()
                    );
                    let hovered = capture(&mut ui, &mut renderer, &theme, cursor);
                    assert_color(fill_pixel(&hovered, target), palette.background.weak.color);
                    save_tabs(
                        &hovered,
                        &bounds,
                        &format!("{locale_name}-{theme_name}-{index}-hover"),
                    );

                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            cursor,
                            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        )
                        .is_empty()
                    );
                    let pressed = capture(&mut ui, &mut renderer, &theme, cursor);
                    assert_color(fill_pixel(&pressed, target), neutral);
                    save_tabs(
                        &pressed,
                        &bounds,
                        &format!("{locale_name}-{theme_name}-{index}-pressed"),
                    );

                    let messages = send(
                        &mut ui,
                        &mut renderer,
                        cursor,
                        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    );
                    assert!(
                        matches!(messages.as_slice(), [Message::General(crate::general::Action::TabSelected(tab))] if *tab == next),
                        "{messages:?}"
                    );
                    drop(ui);
                    let Message::General(crate::general::Action::TabSelected(tab)) =
                        messages.into_iter().next().unwrap()
                    else {
                        panic!("expected tab selection")
                    };
                    crate::general_update::select_tab(
                        &mut app.general_settings_tab,
                        &mut app.themes_settings.error,
                        tab,
                    );
                }
            }
        }
    }
}

use crate::general_fields::{self, GeneralSettingField};
