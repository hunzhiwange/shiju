use super::*;
use crate::app::components::system_settings::{self, SystemTab};
use crate::app::components::system_settings_themes::{self, editor, gallery};
use crate::app::message::settings::{
    SettingsMessage, ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage,
};
use crate::app::state::{GeneralSettingsTab, ThemeEditorTab, ThemesSettingsMode};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::operation::{Operation, Outcome, focusable};
use iced::advanced::{Layout, Shell, clipboard, layout, overlay, renderer, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use vw_config_types::ui::DesktopLocale;

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    app.expect("isolated settings fixture")
}

struct Scene<'a> {
    element: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: Renderer,
    viewport: Rectangle,
}

impl<'a> Scene<'a> {
    async fn new(mut element: Element<'a, Message>, width: u32, height: u32) -> Self {
        static FONTS: std::sync::Once = std::sync::Once::new();
        FONTS.call_once(|| {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        });
        let font = Font::with_name(if cfg!(target_os = "macos") {
            "Noto Sans CJK SC"
        } else {
            "JetBrains Mono"
        });
        let renderer = Renderer::new(font, 16.0.into(), Some("tiny-skia")).await.unwrap();
        let viewport = Rectangle::with_size(Size::new(width as f32, height as f32));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        Self { element, tree, node, renderer, viewport }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.node), |node, &index| node.children().nth(index).unwrap())
            .bounds()
    }

    fn dispatch(&mut self, event: Event, mut cursor: mouse::Cursor) -> Vec<Message> {
        let root = Layout::new(&self.node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            let layout = Layout::new(&node);
            let interaction = layer.mouse_interaction(layout, cursor, &self.renderer);
            layer.update(&event, layout, cursor, &self.renderer, &mut clipboard::Null, &mut shell);
            if interaction != mouse::Interaction::None {
                cursor = mouse::Cursor::Unavailable;
            }
        }
        if !shell.is_event_captured() {
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                root,
                cursor,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &self.viewport,
            );
        }
        messages
    }

    fn overlay_bounds(&mut self) -> Option<Rectangle> {
        self.overlay_bounds_inside(self.viewport)
    }

    fn overlay_bounds_inside(&mut self, clip: Rectangle) -> Option<Rectangle> {
        let mut layer = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        )?;
        let node = layer.as_overlay_mut().layout(&self.renderer, self.viewport.size());
        let mut layout = Layout::new(&node);
        while layout.bounds() == self.viewport || layout.bounds() == clip {
            layout = layout.children().next()?;
        }
        Some(layout.bounds())
    }

    fn click(&mut self, point: Point) -> Vec<Message> {
        [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ]
        .into_iter()
        .flat_map(|event| self.dispatch(Event::Mouse(event), mouse::Cursor::Available(point)))
        .collect()
    }

    fn focus_next(&mut self) {
        let mut operation: Box<dyn Operation> = Box::new(focusable::focus_next());
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                operation.as_mut(),
            );
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                _ => break,
            }
        }
    }

    fn key(&mut self, key: Named) -> Vec<Message> {
        let key = Key::Named(key);
        self.dispatch(
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    fn scroll(&mut self, position: Point, y: f32) -> Vec<Message> {
        self.dispatch(
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
            }),
            mouse::Cursor::Available(position),
        )
    }

    fn hover(&mut self, position: Point) -> Vec<Message> {
        self.dispatch(
            Event::Mouse(mouse::Event::CursorMoved { position }),
            mouse::Cursor::Available(position),
        )
    }

    fn snapshot(&mut self, theme: &Theme, cursor: mouse::Cursor, name: &str) -> Vec<u8> {
        use iced::advanced::Renderer as _;

        assert!(
            self.dispatch(
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
                cursor,
            )
            .is_empty()
        );
        self.renderer.reset(self.viewport);
        let defaults = renderer::Style { text_color: theme.palette().text };
        let root = Layout::new(&self.node);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &defaults,
            root,
            cursor,
            &self.viewport,
        );
        // Rebuild the overlay after event dispatch, as the native UI does.
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            layer.draw(&mut self.renderer, theme, &defaults, Layout::new(&node), cursor);
        }
        let size = Size::new(self.viewport.width as u32, self.viewport.height as u32);
        let pixels = self.renderer.screenshot(size, 1.0, theme.palette().background);
        if !name.is_empty()
            && let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR")
        {
            let file = std::path::PathBuf::from(directory).join(format!("{name}.png"));
            image::save_buffer(file, &pixels, size.width, size.height, image::ColorType::Rgba8)
                .unwrap();
        }
        pixels
    }
}

const TRIGGER: &[usize] = &[0, 2, 0, 1, 0, 0];
const FONT_SLIDER: &[usize] = &[0, 4, 0, 1, 0, 0, 0];
const HEIGHT_SLIDER: &[usize] = &[0, 6, 0, 1, 0, 0, 0];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_05_preserves_values_and_disabled_states() {
    let mut app = fixture();
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;
    app.current_font_size = 16.0;
    app.current_line_height = 24.0;
    app.auto_adjust_line_height = true;
    let mut scene = Scene::new(editor_fields(&app, 250.0), 640, 550).await;
    let trigger = scene.bounds(TRIGGER);
    assert!(scene.click(trigger.center()).is_empty());
    assert!(scene.key(Named::Enter).is_empty());
    assert!(scene.overlay_bounds().is_none(), "following system must disable manual theme");

    let toggle = scene.bounds(&[0, 0, 0, 1, 0, 0]).center();
    assert!(matches!(
        scene.click(toggle).as_slice(),
        [Message::Editor(EditorMessage::ToggleFollowSystemTheme(false))]
    ));

    for (path, maximum) in [(FONT_SLIDER, 30.0), (HEIGHT_SLIDER, 60.0)] {
        let bounds = scene.bounds(path);
        let end = Point::new(bounds.x + bounds.width - 0.1, bounds.center_y());
        let start = Point::new(bounds.x + 0.1, bounds.center_y());
        let mut send =
            |event, point| scene.dispatch(Event::Mouse(event), mouse::Cursor::Available(point));
        let pressed = send(mouse::Event::ButtonPressed(mouse::Button::Left), end);
        let moved = send(mouse::Event::CursorMoved { position: start }, start);
        if maximum == 30.0 {
            assert!(
                matches!(pressed.as_slice(), [Message::Editor(EditorMessage::FontSizePreview(v))] if *v == maximum)
            );
            assert!(matches!(
                moved.as_slice(),
                [Message::Editor(EditorMessage::FontSizePreview(10.0))]
            ));
        } else {
            assert!(
                matches!(pressed.as_slice(), [Message::Editor(EditorMessage::LineHeightPreview(v))] if *v == maximum)
            );
            assert!(matches!(
                moved.as_slice(),
                [Message::Editor(EditorMessage::LineHeightPreview(10.0))]
            ));
        }
        assert!(matches!(
            send(mouse::Event::ButtonReleased(mouse::Button::Left), start).as_slice(),
            [Message::Editor(EditorMessage::CommitMetrics)]
        ));
    }
    drop(scene);

    app.editor_follow_system_theme = false;
    let mut scene = Scene::new(editor_fields(&app, 250.0), 640, 540).await;
    let trigger = scene.bounds(TRIGGER);
    let label_column = scene.bounds(&[0, 2, 0, 0]);
    assert!((trigger.center_y() - label_column.center_y()).abs() < 0.5);
    assert_eq!(trigger.height, 38.0);
    for index in [0, 1] {
        let center = scene.bounds(&[0, 2, 0, 1, 0, 0, 0, index]).center_y();
        assert!((center - trigger.center_y()).abs() < 0.5);
    }

    scene.focus_next();
    assert!(scene.key(Named::Enter).is_empty());
    let menu = scene.overlay_bounds().expect("focused theme opens with Enter");
    assert!(menu.x >= 0.0 && menu.y >= 0.0);
    assert!(menu.x + menu.width <= 640.0 && menu.y + menu.height <= 540.0);
    let first = Point::new(menu.center_x(), menu.y + 24.0);
    let second = Point::new(menu.center_x(), menu.y + 60.0);
    assert!(scene.hover(first).is_empty());
    let first_frame =
        scene.snapshot(&Theme::Dark, mouse::Cursor::Available(first), "editor-menu-first");
    assert!(scene.hover(second).is_empty());
    let second_frame =
        scene.snapshot(&Theme::Dark, mouse::Cursor::Available(second), "editor-menu-second");
    assert!(first_frame != second_frame, "moving between options must update the hover row");
    assert!(scene.hover(Point::ORIGIN).is_empty());
    let outside =
        scene.snapshot(&Theme::Dark, mouse::Cursor::Available(Point::ORIGIN), "editor-menu-exit");
    assert!(outside != second_frame, "moving outside clears hover");
    assert!(scene.scroll(second, -50.0).is_empty());
    assert!(scene.key(Named::Escape).is_empty());
    assert!(scene.overlay_bounds().is_none());

    assert!(scene.click(trigger.center()).is_empty());
    assert!(scene.key(Named::Home).is_empty());
    assert!(scene.key(Named::ArrowDown).is_empty());
    assert!(matches!(
        scene.key(Named::Enter).as_slice(),
        [Message::Editor(EditorMessage::ThemeChanged(theme))] if theme == &Theme::ALL[1]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_05_layout_and_interaction() {
    let mut app = fixture();
    let palette = gallery::palette_hex_from_theme(&Theme::Dark);
    let name = "測試主题Theme".repeat(7);
    let mut draft = editor::new_theme_draft(&[]);
    draft.name = name.clone();
    draft.cover_relative = Some(format!("images/{}.png", "资源圖LongAsset".repeat(6)));
    draft.chrome_chat_header_image_fit = "contain".to_owned();
    let entries = vec![
        gallery::ThemeGalleryEntry {
            id: "fixture-theme".to_owned(),
            name,
            tagline: Some("深色调色板預覽 ".repeat(3)),
            source: gallery::ThemeGallerySource::Custom,
            palette,
            cover_path: None,
            modified: false,
        };
        7
    ];
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        app.app_theme = theme.clone();
        for (locale, locale_name) in [
            (DesktopLocale::SimplifiedChinese, "zh"),
            (DesktopLocale::TraditionalChinese, "zh-hant"),
            (DesktopLocale::English, "en"),
        ] {
            app.desktop_locale = locale;
            for width in [640, 960] {
                Scene::new(view(&app), width, 640).await.snapshot(
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("editor-{locale_name}-{theme_name}-{width}"),
                );
            }

            app.themes_settings.mode = ThemesSettingsMode::List;
            app.themes_settings.entries.clear();
            app.themes_settings.error = Some("Fixture: theme catalog unavailable".into());
            Scene::new(system_settings_themes::view(&app), 640, 640).await.snapshot(
                &theme,
                mouse::Cursor::Unavailable,
                &format!("gallery-empty-{locale_name}-{theme_name}"),
            );
            app.themes_settings.error = None;
            app.themes_settings.entries.clone_from(&entries);
            for width in [640, 960] {
                let mut scene = Scene::new(system_settings_themes::view(&app), width, 1120).await;
                scene.snapshot(
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("gallery-{locale_name}-{theme_name}-{width}"),
                );
                let grid = scene.bounds(&[3]);
                let first = scene.bounds(&[3, 0]);
                let second = scene.bounds(&[3, 1]);
                assert!(grid.width <= width as f32 && first.width >= 200.0);
                assert!(second.x > first.x && second.y == first.y);
            }

            app.show_system_settings = true;
            app.system_settings_tab = SystemTab::General;
            app.general_settings_tab = GeneralSettingsTab::Themes;
            app.themes_settings.mode = ThemesSettingsMode::Edit;
            app.themes_settings.draft = Some(draft.clone());
            for tab in [
                ThemeEditorTab::Basics,
                ThemeEditorTab::Palette,
                ThemeEditorTab::Assets,
                ThemeEditorTab::Chrome,
                ThemeEditorTab::Preview,
            ] {
                app.themes_settings.editor_tab = tab;
                let mut scene = Scene::new(system_settings::view(&app), 420, 650).await;
                scene.snapshot(
                    &theme,
                    mouse::Cursor::Unavailable,
                    &format!("theme-modal-{tab:?}-{locale_name}-{theme_name}-420"),
                );
            }
        }
    }

    app.desktop_locale = DesktopLocale::English;
    app.themes_settings.editor_tab = ThemeEditorTab::Assets;
    app.themes_settings.error = Some("Fixture: invalid image file".into());
    let mut scene = Scene::new(editor::view(&app), 420, 620).await;
    let header = scene.bounds(&[0, 0]);
    let fields = scene.bounds(&[0, 1]);
    let footer = scene.bounds(&[0, 2]);
    let save = scene.bounds(&[0, 2, 1, 0]).center();
    let cancel = scene.bounds(&[0, 2, 1, 1]).center();
    assert!(header.y + header.height <= fields.y);
    assert!(fields.y + fields.height <= footer.y);
    assert!(footer.y + footer.height <= 620.0);
    let before =
        scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "theme-assets-before-scroll");
    assert!(scene.scroll(fields.center(), -430.0).is_empty());
    let after =
        scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "theme-assets-after-scroll");
    assert!(before != after, "the resource form scrolls");
    let footer_pixels = (footer.y.ceil() as usize * 420 * 4)..;
    assert!(before[footer_pixels.clone()] == after[footer_pixels], "footer remains fixed");
    assert!(scene.key(Named::Enter).is_empty());
    assert!(matches!(
        scene.click(cancel).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::CancelEdit))]
    ));
    assert!(matches!(
        scene.click(save).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::SaveDraft))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_05_gallery_uses_original_theme_id_and_paging_messages() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.themes_settings.entries = vec![
        gallery::ThemeGalleryEntry {
            id: "fixture-id".to_owned(),
            name: "Fixture name".to_owned(),
            tagline: None,
            source: gallery::ThemeGallerySource::Custom,
            palette: gallery::palette_hex_from_theme(&Theme::Light),
            cover_path: None,
            modified: false,
        };
        7
    ];
    let mut scene = Scene::new(gallery::view(&app), 960, 760).await;
    let apply = scene.bounds(&[3, 0, 0, 2, 0]).center();
    let prev = scene.bounds(&[4, 0]).center();
    let next = scene.bounds(&[4, 2]).center();
    assert!(scene.click(prev).is_empty());
    assert!(matches!(
        scene.click(next).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::GalleryNextPage))]
    ));
    assert!(matches!(
        scene.click(apply).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ApplyCustom(id)))] if id == "fixture-id"
    ));
    drop(scene);

    app.active_custom_theme_id = Some("fixture-id".to_owned());
    let mut scene = Scene::new(gallery::view(&app), 960, 760).await;
    let apply = scene.bounds(&[3, 0, 0, 2, 0]).center();
    assert!(scene.click(apply).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_05_image_fit_keeps_canonical_value() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    app.themes_settings.mode = ThemesSettingsMode::Edit;
    app.themes_settings.editor_tab = ThemeEditorTab::Assets;
    let mut draft = editor::new_theme_draft(&[]);
    draft.chrome_chat_header_image_fit = "contain".into();
    app.themes_settings.draft = Some(draft);
    let mut scene = Scene::new(editor::view(&app), 420, 620).await;
    let viewport = scene.bounds(&[0, 1]);
    let trigger = scene.bounds(&[0, 1, 0, 0, 2, 1, 0, 4, 0, 1, 0, 0]);
    let offset = trigger.center_y() - viewport.center_y();
    assert!(scene.scroll(viewport.center(), -offset).is_empty());
    let point = Point::new(trigger.center_x(), viewport.center_y());
    assert!(scene.click(point).is_empty());
    let menu = scene.overlay_bounds().expect("image fit menu in the scrolled form");
    assert!(
        menu.x >= 0.0
            && menu.y >= viewport.y
            && menu.y + menu.height <= viewport.y + viewport.height
    );
    scene.snapshot(&Theme::Light, mouse::Cursor::Available(point), "theme-assets-fit-menu");
    assert!(scene.key(Named::ArrowDown).is_empty());
    assert!(matches!(
        scene.key(Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftChromeChanged(ThemeChromeField::ChatHeaderImageFit, value)))] if value == "fill"
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_05_theme_name_and_color_picker_keep_original_messages() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.themes_settings.mode = ThemesSettingsMode::Edit;
    app.themes_settings.draft = Some(editor::new_theme_draft(&[]));
    let mut scene = Scene::new(editor::view(&app), 420, 650).await;
    let input = scene.bounds(&[0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0]);
    let text = scene.bounds(&[0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0, 0]);
    assert!((input.center_y() - text.center_y()).abs() < 0.5);
    let id = scene.bounds(&[0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0]);
    let ime_commit = Event::InputMethod(iced::advanced::input_method::Event::Commit("示例".into()));
    assert!(scene.click(id.center()).is_empty());
    assert!(scene.dispatch(ime_commit.clone(), mouse::Cursor::Unavailable).is_empty());
    assert!(scene.click(input.center()).is_empty());
    assert!(matches!(
        scene.dispatch(ime_commit, mouse::Cursor::Unavailable).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftNameChanged(value)))] if value == "示例"
    ));
    drop(scene);

    app.themes_settings.editor_tab = ThemeEditorTab::Palette;
    let target = ThemeColorPickerTarget::Palette(ThemePaletteField::Background);
    let mut scene = Scene::new(editor::view(&app), 420, 650).await;
    let swatch = scene.bounds(&[0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);
    assert!(matches!(
        scene.click(swatch.center()).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerToggled(value)))] if *value == target
    ));
    drop(scene);

    app.themes_settings.color_picker_target = Some(target);
    let mut scene = Scene::new(editor::view(&app), 420, 650).await;
    let scroll = scene.bounds(&[0, 1]);
    let picker = scene.overlay_bounds_inside(scroll).expect("color picker popover");
    assert!(picker.y >= scroll.y && picker.y + picker.height <= scroll.y + scroll.height);
    scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "theme-color-picker-zh-dark-420");
    let confirm = Point::new(picker.x + 36.0, picker.y + picker.height - 30.0);
    assert!(matches!(
        scene.click(confirm).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerClosed))]
    ));
    let save = scene.bounds(&[0, 2, 0, 0]).center();
    assert!(matches!(
        scene.click(save).as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerClosed))]
    ));
}

#[derive(Default)]
struct ThemeText(Vec<(String, Rectangle)>);

impl Operation for ThemeText {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.to_owned(), bounds));
    }
}

impl Scene<'_> {
    fn theme_text(&mut self) -> ThemeText {
        let mut text = ThemeText::default();
        self.element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &mut text,
        );
        text
    }

    fn text_bounds(&mut self, expected: &str) -> Rectangle {
        self.theme_text()
            .0
            .into_iter()
            .find_map(|(value, bounds)| (value == expected).then_some(bounds))
            .unwrap_or_else(|| panic!("missing text: {expected}"))
    }

    fn assert_hidden(&mut self, expected: &str) {
        assert!(
            !self.theme_text().0.iter().any(|(value, _)| value.contains(expected)),
            "static help is still in the body: {expected}"
        );
    }

    fn assert_theme_help(&mut self, theme: &Theme, title: Rectangle, expected: &[&str]) {
        for text in expected {
            self.assert_hidden(text);
        }
        let point = Point::new(title.x + title.width + 15.0, title.center_y());
        let cursor = mouse::Cursor::Available(point);
        let idle = self.snapshot(theme, cursor, "");
        assert!(self.hover(point).is_empty());
        let card = self.overlay_bounds().expect("hover opens the help card");
        assert!(card.x >= 0.0 && card.y >= 0.0);
        assert!(card.x + card.width <= self.viewport.width);
        assert!(card.y + card.height <= self.viewport.height);
        assert!(self.snapshot(theme, cursor, "") != idle);
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        assert!(self.overlay_bounds().is_none());

        assert!(self.click(point).is_empty());
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        let card = self.overlay_bounds().expect("click pins the help card");
        let start = Point::new(card.x + 12.5, card.y + 15.0);
        let end = Point::new(card.x + card.width - 1.0, card.y + card.height - 1.0);
        for (position, event) in [
            (start, mouse::Event::ButtonPressed(mouse::Button::Left)),
            (end, mouse::Event::CursorMoved { position: end }),
            (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            assert!(
                self.dispatch(Event::Mouse(event), mouse::Cursor::Available(position)).is_empty()
            );
        }
        let copy = Key::Character("c".into());
        let copied = self.dispatch(
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: copy.clone(),
                modified_key: copy,
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
                location: keyboard::Location::Standard,
                modifiers: if cfg!(target_os = "macos") {
                    keyboard::Modifiers::LOGO
                } else {
                    keyboard::Modifiers::CTRL
                },
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        );
        assert!(
            matches!(copied.as_slice(), [Message::CopyCode(value)] if expected.iter().all(|text| value.contains(text))),
            "the card contains selectable localized help: {copied:?}"
        );
        assert!(self.key(Named::Escape).is_empty());
        assert!(self.overlay_bounds().is_none(), "Escape closes the pinned help");
    }
}

struct TestThemesRoot(tempfile::TempDir);

impl TestThemesRoot {
    fn new() -> Self {
        use crate::app::theme_catalog::{ensure_bundled_presets, set_test_themes_root};
        let dir = tempfile::tempdir().unwrap();
        ensure_bundled_presets(dir.path()).unwrap();
        set_test_themes_root(Some(dir.path().to_path_buf()));
        Self(dir)
    }
}

impl Drop for TestThemesRoot {
    fn drop(&mut self) {
        crate::app::theme_catalog::set_test_themes_root(None);
    }
}

fn handle_theme_message(app: &mut App, messages: Vec<Message>) {
    let [Message::Settings(message @ SettingsMessage::Themes(_))] = messages.as_slice() else {
        panic!("expected a theme action, got {messages:?}");
    };
    let _ = crate::app::message::settings::update(app, message.clone());
}

const GALLERY_RULE: &str =
    "所有主题均可导出；预设主题可编辑并恢复默认、不可删除，自定义主题可编辑或删除。";
const BUILTIN_RULE: &str = "修改将保存为本地覆盖层，可随时恢复 Iced 内置默认值。";
const PRESET_RULE: &str = "此预设可编辑并随时恢复嵌入默认值，但不可删除。";
const ID_RULE: &str = "由名称自动生成；冲突时追加后缀";
const THEME_HELP_LAYOUTS: [(Theme, u32); 4] =
    [(Theme::Light, 960), (Theme::Dark, 960), (Theme::Light, 420), (Theme::Dark, 420)];

#[tokio::test]
async fn settings_help_gap_02_gallery_rules_and_card_status() {
    use crate::app::i18n::I18n;
    use gallery::{ThemeGalleryEntry, ThemeGallerySource};

    let mut app = fixture();
    app.themes_settings.entries = vec![
        ThemeGalleryEntry {
            id: "Light".into(),
            name: "Light".into(),
            tagline: None,
            source: ThemeGallerySource::Builtin,
            palette: gallery::palette_hex_from_theme(&Theme::Light),
            cover_path: None,
            modified: true,
        },
        ThemeGalleryEntry {
            id: "custom-fixture".into(),
            name: "Custom fixture".into(),
            tagline: Some("Fixture tagline".into()),
            source: ThemeGallerySource::Custom,
            palette: gallery::palette_hex_from_theme(&Theme::Light),
            cover_path: None,
            modified: false,
        },
    ];
    app.active_custom_theme_id = Some("custom-fixture".into());
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        app.desktop_locale = locale;
        for (theme, width) in THEME_HELP_LAYOUTS {
            app.app_theme = theme.clone();
            let mut scene = Scene::new(system_settings_themes::view(&app), width, 880).await;
            scene.assert_hidden(i18n.__(GALLERY_RULE));
            scene.assert_hidden(i18n.__("本地覆盖的内置主题"));
            for status in
                [i18n.__("内置"), i18n.__("已修改"), i18n.__("当前应用"), "Fixture tagline"]
            {
                scene.text_bounds(status);
            }
            let title = scene.text_bounds(i18n.__("主题画廊"));
            scene.assert_theme_help(
                &theme,
                title,
                &[i18n.__(GALLERY_RULE), i18n.__("内置主题的编辑保存为本地覆盖，可恢复默认。")],
            );
            let export = scene.bounds(&[3, 0, 0, 2, 1]).center();
            assert!(matches!(
                scene.click(export).as_slice(),
                [Message::Settings(SettingsMessage::Themes(ThemesMessage::ExportPack(id)))] if id == "Light"
            ));
        }
    }
}

#[tokio::test]
async fn settings_help_gap_02_gallery_opens_builtin_and_preset_editors() {
    use crate::app::i18n::I18n;
    use crate::app::theme_catalog::{ThemeStore, list_bundled_preset_ids};
    use gallery::ThemeGallerySource;

    let mut app = fixture();
    let root = TestThemesRoot::new();
    let store = ThemeStore::new(root.0.path());
    let entries = gallery::build_gallery_entries(Some(&store)).unwrap();
    let preset_id = list_bundled_preset_ids().remove(0);
    let builtin = entries
        .iter()
        .find(|entry| entry.id == "Dark" && entry.source == ThemeGallerySource::Builtin)
        .unwrap();
    let preset = entries.iter().find(|entry| entry.id == preset_id).unwrap();

    for (entry, title, rule) in
        [(builtin, "编辑内置主题", BUILTIN_RULE), (preset, "编辑主题", PRESET_RULE)]
    {
        for locale in [
            DesktopLocale::SimplifiedChinese,
            DesktopLocale::TraditionalChinese,
            DesktopLocale::English,
        ] {
            let i18n = I18n::new(locale);
            app.desktop_locale = locale;
            for (theme, width) in THEME_HELP_LAYOUTS {
                app.app_theme = theme.clone();
                app.themes_settings.entries = vec![entry.clone()];
                let mut scene = Scene::new(system_settings_themes::view(&app), width, 800).await;
                let edit = scene.text_bounds(i18n.__("编辑")).center();
                let messages = scene.click(edit);
                drop(scene);
                handle_theme_message(&mut app, messages);
                assert_eq!(app.themes_settings.mode, ThemesSettingsMode::Edit);

                let gallery = system_settings_themes::view(&app);
                let mut scene =
                    Scene::new(system_settings_themes::editor_overlay(&app, gallery), width, 800)
                        .await;
                scene.assert_hidden(i18n.__(rule));
                scene.text_bounds(i18n.__("标识只读"));
                if entry.source == ThemeGallerySource::Preset {
                    scene.text_bounds(i18n.__("预设不可删除。"));
                    assert!(
                        !scene.theme_text().0.iter().any(|(value, _)| value == i18n.__("删除"))
                    );
                }
                let title = scene.text_bounds(i18n.__(title));
                if entry.source == ThemeGallerySource::Preset {
                    scene.assert_theme_help(
                        &theme,
                        title,
                        &[i18n.__("此预设可编辑并随时恢复嵌入默认值。")],
                    );
                } else {
                    let derivation = i18n.__("内置覆盖层标识由主题名派生。");
                    scene.assert_hidden(derivation);
                    scene.assert_theme_help(&theme, title, &[i18n.__(rule), derivation]);
                }
                let restore = scene.text_bounds(i18n.__("恢复默认")).center();
                let messages = scene.click(restore);
                match (entry.source, messages.as_slice()) {
                    (
                        ThemeGallerySource::Builtin,
                        [
                            Message::Settings(SettingsMessage::Themes(
                                ThemesMessage::RestoreBuiltinDefault(id),
                            )),
                        ],
                    )
                    | (
                        ThemeGallerySource::Preset,
                        [
                            Message::Settings(SettingsMessage::Themes(
                                ThemesMessage::RestorePresetDefault(id),
                            )),
                        ],
                    ) => assert_eq!(id, &entry.id),
                    _ => panic!("unexpected restore action: {messages:?}"),
                }
                drop(scene);
                handle_theme_message(&mut app, messages);
                assert_eq!(app.themes_settings.mode, ThemesSettingsMode::List);
            }
        }
    }
}

#[tokio::test]
async fn settings_help_gap_02_creation_help_keeps_validation_and_picker() {
    use crate::app::i18n::I18n;

    let mut app = fixture();
    let _root = TestThemesRoot::new();
    let i18n = I18n::new(DesktopLocale::SimplifiedChinese);
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.app_theme = Theme::Dark;
    let mut scene = Scene::new(system_settings_themes::view(&app), 420, 800).await;
    let new = scene.text_bounds(i18n.__("新建主题")).center();
    let messages = scene.click(new);
    drop(scene);
    handle_theme_message(&mut app, messages);

    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    scene.assert_hidden(i18n.__(ID_RULE));
    scene.assert_hidden(i18n.__("名称必填；新建时标识由名称生成。"));
    scene.text_bounds(i18n.__("名称必填。"));
    let id = scene.text_bounds(i18n.__("标识"));
    scene.assert_theme_help(&Theme::Dark, id, &[i18n.__(ID_RULE)]);
    let chrome = scene.text_bounds(i18n.__("控件皮肤")).center();
    let messages = scene.click(chrome);
    drop(scene);
    handle_theme_message(&mut app, messages);

    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    scene.assert_hidden(i18n.__("可选覆盖；非法 hex 会阻止保存。"));
    scene.text_bounds(i18n.__("非法 hex 会阻止保存。"));
    let heading = scene.bounds(&[0, 1, 0, 0, 0, 0, 0]);
    scene.assert_theme_help(&Theme::Dark, heading, &[i18n.__("可选覆盖。")]);
    let palette = scene.text_bounds(i18n.__("调色板")).center();
    let messages = scene.click(palette);
    drop(scene);
    handle_theme_message(&mut app, messages);

    let target = ThemeColorPickerTarget::Palette(ThemePaletteField::Background);
    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    scene.text_bounds(i18n.__("仅支持 #rrggbb 十六进制颜色。"));
    let swatch = scene.bounds(&[0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]).center();
    let messages = scene.click(swatch);
    assert!(matches!(messages.as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerToggled(value)))] if *value == target));
    drop(scene);
    handle_theme_message(&mut app, messages);

    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    let scroll = scene.bounds(&[0, 1]);
    scene.overlay_bounds_inside(scroll).expect("color picker popover");
    scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "theme-help-gap-picker");
    let cancel = scene.text_bounds(i18n.__("取消")).center();
    let messages = scene.click(cancel);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Themes(ThemesMessage::ColorPickerClosed))]
    ));
    drop(scene);
    handle_theme_message(&mut app, messages);

    handle_theme_message(
        &mut app,
        vec![Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftPaletteChanged(
            ThemePaletteField::Background,
            "invalid-color".into(),
        )))],
    );
    handle_theme_message(
        &mut app,
        vec![Message::Settings(SettingsMessage::Themes(ThemesMessage::DraftNameChanged(
            "Fixture theme".into(),
        )))],
    );
    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    let save = scene.text_bounds(i18n.__("保存")).center();
    let messages = scene.click(save);
    drop(scene);
    handle_theme_message(&mut app, messages);
    let error = app.themes_settings.error.as_deref().expect("invalid color rejected");
    let mut scene = Scene::new(editor::view(&app), 420, 680).await;
    scene.text_bounds(error);
    scene.snapshot(&Theme::Dark, mouse::Cursor::Unavailable, "");
    let cancel = scene.text_bounds(i18n.__("取消")).center();
    let messages = scene.click(cancel);
    drop(scene);
    handle_theme_message(&mut app, messages);
    assert!(app.themes_settings.draft.is_none());

    let mut empty = Scene::new(editor::view(&app), 420, 680).await;
    empty.text_bounds(i18n.__("当前没有可编辑的草稿。"));
}
