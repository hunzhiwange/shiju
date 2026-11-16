#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code, unused_imports)]
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

use crate::views::modals::{file_tree_delete, rename};
#[derive(Debug, Clone)]
enum Message {
    Rename(rename::Event),
    Delete(file_tree_delete::Event),
    CloseError,
}
struct RenameInput {
    desktop_locale: DesktopLocale,
    session_rename_id: Option<String>,
    session_rename_value: String,
    sessions: Vec<session_info::Info>,
    project_sessions: std::collections::HashMap<String, Vec<session_info::Info>>,
    pending: Option<(String, Vec<String>)>,
}
fn fixture() -> RenameInput {
    RenameInput {
        desktop_locale: DesktopLocale::SimplifiedChinese,
        session_rename_id: None,
        session_rename_value: String::new(),
        sessions: Vec::new(),
        project_sessions: Default::default(),
        pending: None,
    }
}
fn with_session_rename<'a>(
    input: &'a RenameInput,
    root: Element<'a, Message>,
) -> Element<'a, Message> {
    rename::with_session_rename(
        input.session_rename_id.as_deref(),
        &input.session_rename_value,
        &input.sessions,
        &input.project_sessions,
        input.desktop_locale,
        Message::Rename,
        root,
    )
}
fn with_file_tree_delete_confirm<'a>(
    input: &'a RenameInput,
    root: Element<'a, Message>,
) -> Element<'a, Message> {
    file_tree_delete::with_file_tree_delete_confirm(
        input.pending.as_ref().map(|(_, paths)| paths.as_slice()),
        input.desktop_locale,
        Message::Delete,
        root,
    )
}
async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn root() -> Element<'static, Message> {
    container(button("underlay").on_press(Message::CloseError))
        .width(iced::Fill)
        .height(iced::Fill)
        .into()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Unavailable,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            cursor,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        )
        .is_empty()
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_32_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scroll: Vec<(Rectangle, iced::Vector)>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scroll.push((bounds, translation));
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn label(geometry: &Geometry, value: &str) -> Rectangle {
    geometry
        .labels
        .iter()
        .find(|(label, _)| label == value)
        .unwrap_or_else(|| panic!("missing {value}: {:?}", geometry.labels))
        .1
}

fn assert_fits(geometry: &Geometry, size: Size<u32>) {
    for (value, bounds) in geometry.labels.iter().chain(&geometry.inputs) {
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{value}: {bounds:?}");
        assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{value}: {bounds:?}");
        assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{value}: {bounds:?}");
        assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{value}: {bounds:?}");
    }
}

fn find_layout<'a>(
    layout: Layout<'a>,
    matches: impl Fn(Rectangle) -> bool + Copy,
) -> Option<Layout<'a>> {
    if matches(layout.bounds()) {
        return Some(layout);
    }
    layout.children().find_map(|child| find_layout(child, matches))
}

fn assert_control_alignment(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    size: Size<u32>,
    input: Option<Rectangle>,
) -> Point {
    let mut tree = Tree::new(view.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(size.width as f32, size.height as f32));
    let node = view.as_widget_mut().layout(&mut tree, renderer, &limits);
    let layout = Layout::new(&node);
    let close = find_layout(layout, |bounds| bounds.width == 28.0 && bounds.height == 28.0)
        .expect("close button");
    let icon = find_layout(close, |bounds| bounds.width == 12.0 && bounds.height == 12.0)
        .expect("close icon");
    assert!((close.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    if let Some(input) = input {
        let field = find_layout(layout, |bounds| bounds == input).expect("input field");
        let glyphs = field.children().next().expect("input text");
        assert!((glyphs.bounds().center_y() - input.center_y()).abs() < 0.5);
    }
    close.bounds().center()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_preserves_values_and_disabled_states() {
    let mut app = fixture();
    let mut renderer = renderer().await;
    let long_title = "长會話名稱example".repeat(28);
    let project_dir = tempfile::tempdir().unwrap();
    app.session_rename_id = Some("session-fixture".into());
    app.session_rename_value = DRAFT.into();
    let session = serde_json::from_value(serde_json::json!({
        "id": "session-fixture", "slug": "fixture", "projectID": "fixture",
        "directory": project_dir.path(), "title": long_title, "version": "1",
        "time": {"created": 1, "updated": 2}
    }))
    .unwrap();
    app.sessions = vec![session];

    for (locale, size, language) in [
        (DesktopLocale::SimplifiedChinese, VIEW_SIZE, "zh-cn"),
        (DesktopLocale::TraditionalChinese, Size::new(640, 540), "zh-tw"),
        (DesktopLocale::English, Size::new(960, 600), "en"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            {
                let (view, kind) = (with_session_rename(&app, root()), "session");
                let mut ui = scene(view, &mut renderer, size);
                let bounds = geometry(&mut ui, &renderer);
                assert_fits(&bounds, size);
                let [(value, input)] = bounds.inputs.as_slice() else {
                    panic!("expected one input")
                };
                assert_eq!(value, DRAFT);
                let view = with_session_rename(&app, root());
                assert_control_alignment(view, &renderer, size, Some(*input));
                if kind == "session" {
                    let [(viewport, translation)] = bounds.scroll.as_slice() else {
                        panic!("expected a scrollable session title");
                    };
                    assert_eq!(translation.y, 0.0);
                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            mouse::Cursor::Available(viewport.center()),
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
                            }),]
                        )
                        .is_empty()
                    );
                    let after = geometry(&mut ui, &renderer);
                    assert!(after.scroll[0].1.y > 0.0);
                    assert_eq!(after.inputs, bounds.inputs);
                    assert_eq!(
                        label(&after, i18n.__("会话重命名")),
                        label(&bounds, i18n.__("会话重命名"))
                    );
                    assert_eq!(label(&after, i18n.__("确定")), label(&bounds, i18n.__("确定")));
                    capture(
                        &mut ui,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("{language}-{mode}-session-scrolled"),
                        mouse::Cursor::Unavailable,
                    );
                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            mouse::Cursor::Available(viewport.center()),
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: 8.0 },
                            }),]
                        )
                        .is_empty()
                    );
                }
                assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
                assert!(click(&mut ui, &mut renderer, Point::new(5.0, 5.0)).is_empty());
                assert!(
                    click(
                        &mut ui,
                        &mut renderer,
                        Point::new(input.x + input.width - 8.0, input.center_y())
                    )
                    .is_empty()
                );
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-{kind}"),
                    mouse::Cursor::Unavailable,
                );
                let changed = key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x"));
                assert!(matches!((kind, changed.as_slice()),
                    ("session", [Message::Rename(rename::Event::SessionRenameChanged(value))])
                        if value == &format!("{DRAFT}x")));
                let messages = key(&mut ui, &mut renderer, Key::Named(Named::Enter), None);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("session", [Message::Rename(rename::Event::SessionRenameSave)])
                ));
                let save = label(&bounds, i18n.__("确定")).center();
                let messages = click(&mut ui, &mut renderer, save);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("session", [Message::Rename(rename::Event::SessionRenameSave)])
                ));
                let cancel = label(&bounds, i18n.__("取消")).center();
                let messages = click(&mut ui, &mut renderer, cancel);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("session", [Message::Rename(rename::Event::SessionRenameCancel)])
                ));
            }
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_tree_delete_confirmation_draws_and_routes_actions() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let path = "/fixture/项目/待删除的文件.txt";
    app.pending =
        Some(("/fixture/项目".into(), vec![path.into(), "/fixture/项目/另一个文件.txt".into()]));
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;

    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = scene(with_file_tree_delete_confirm(&app, root()), &mut renderer, VIEW_SIZE);
        let bounds = geometry(&mut ui, &renderer);
        assert_fits(&bounds, VIEW_SIZE);
        label(&bounds, path);
        label(&bounds, "/fixture/项目/另一个文件.txt");
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            VIEW_SIZE,
            "file-tree-delete",
            mouse::Cursor::Unavailable,
        );
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&bounds, i18n.__("删除")).center()).as_slice(),
            [Message::Delete(file_tree_delete::Event::FileTreeDeleteConfirmed)]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&bounds, i18n.__("取消")).center()).as_slice(),
            [Message::Delete(file_tree_delete::Event::FileTreeDeleteCanceled)]
        ));
        assert!(matches!(
            key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).as_slice(),
            [Message::Delete(file_tree_delete::Event::FileTreeDeleteCanceled)]
        ));
    }
}

const DRAFT: &str = "新草稿・新檔案・draft";
const VIEW_SIZE: Size<u32> = Size::new(420, 480);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_absent_modals_preserve_underlay() {
    let mut app = fixture();
    let mut renderer = renderer().await;
    app.session_rename_id = None;
    for view in [with_session_rename(&app, root())] {
        let mut ui = scene(view, &mut renderer, VIEW_SIZE);
        let bounds = geometry(&mut ui, &renderer);
        assert!(bounds.inputs.is_empty());
        assert_eq!(bounds.labels.len(), 1);
        let underlay = label(&bounds, "underlay").center();
        assert!(matches!(
            click(&mut ui, &mut renderer, underlay).as_slice(),
            [Message::CloseError]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_escape_cancels_only_the_visible_modal() {
    let mut app = fixture();
    app.session_rename_id = Some("session-fixture".into());
    app.session_rename_value = DRAFT.into();
    let mut renderer = renderer().await;
    let mut ui = scene(with_session_rename(&app, root()), &mut renderer, VIEW_SIZE);
    if let Some((_, input)) = geometry(&mut ui, &renderer).inputs.first() {
        assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    }
    let messages = key(&mut ui, &mut renderer, Key::Named(Named::Escape), None);
    assert!(
        matches!(messages.as_slice(), [Message::Rename(rename::Event::SessionRenameCancel)]),
        "Escape did not cancel session"
    );
}
