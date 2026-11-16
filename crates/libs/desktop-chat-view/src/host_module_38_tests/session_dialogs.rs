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

use crate::modals::{chat_fork, chat_reset};
use desktop_session::message::chat::{
    events::ForkSessionTarget,
    fork::{ForkSessionAnchor, ForkSessionDialog},
};
#[derive(Debug, Clone)]
enum Message {
    Fork(chat_fork::Event),
    Reset(chat_reset::Event),
    CloseError,
}
struct Dialogs {
    desktop_locale: DesktopLocale,
    project_is_git_repo: bool,
    dialog: Option<ForkSessionDialog>,
    reset: Option<usize>,
}
fn fixture() -> Dialogs {
    Dialogs {
        desktop_locale: DesktopLocale::SimplifiedChinese,
        project_is_git_repo: true,
        dialog: None,
        reset: None,
    }
}
fn prepare_chat(input: &mut Dialogs) {
    input.dialog = Some(ForkSessionDialog {
        anchor: ForkSessionAnchor {
            session_id: "source".into(),
            directory: "/fixture/source".into(),
            user_message_id: "second-turn".into(),
            open_generation: 0,
        },
        prompt: String::new(),
        in_progress: false,
    });
    input.reset = Some(1);
}
fn with_chat_fork_dialog<'a>(input: &Dialogs, root: Element<'a, Message>) -> Element<'a, Message> {
    chat_fork::with_chat_fork_dialog(
        input.dialog.as_ref(),
        input.project_is_git_repo,
        input.desktop_locale,
        Message::Fork,
        root,
    )
}
fn with_chat_reset_dialog<'a>(input: &Dialogs, root: Element<'a, Message>) -> Element<'a, Message> {
    chat_reset::with_chat_reset_dialog(input.reset, 3, input.desktop_locale, Message::Reset, root)
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
async fn fork_desktop_layout_and_interaction() {
    let mut app = fixture();
    prepare_chat(&mut app);
    let mut renderer = renderer().await;

    app.project_is_git_repo = false;
    {
        let mut fork = scene(with_chat_fork_dialog(&app, root()), &mut renderer, VIEW_SIZE);
        let bounds = geometry(&mut fork, &renderer);
        let i18n = I18n::new(app.desktop_locale);
        label(&bounds, i18n.__("在当前目录中打开新会话"));
        assert!(!bounds.labels.iter().any(|(label, _)| label == i18n.__("分叉到新工作树")));
    }
    app.project_is_git_repo = true;

    for (locale, size, language) in [
        (DesktopLocale::SimplifiedChinese, VIEW_SIZE, "zh-cn"),
        (DesktopLocale::TraditionalChinese, Size::new(640, 540), "zh-tw"),
        (DesktopLocale::English, Size::new(960, 600), "en"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            let mut fork = scene(with_chat_fork_dialog(&app, root()), &mut renderer, size);
            let bounds = geometry(&mut fork, &renderer);
            assert_fits(&bounds, size);
            let local = label(&bounds, i18n.__("在当前目录中打开新会话")).center();
            let worktree = label(&bounds, i18n.__("分叉到新工作树")).center();
            let idle = capture(
                &mut fork,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-fork"),
                mouse::Cursor::Unavailable,
            );
            for (point, target) in [(local, "local"), (worktree, "worktree")] {
                let cursor = mouse::Cursor::Available(point);
                assert!(
                    send(
                        &mut fork,
                        &mut renderer,
                        cursor,
                        &[
                            Event::Mouse(mouse::Event::CursorMoved { position: point }),
                            Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
                            }),
                        ]
                    )
                    .is_empty()
                );
                assert!(
                    capture(
                        &mut fork,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("{language}-{mode}-fork-{target}"),
                        cursor,
                    ) != idle
                );
            }
            assert!(
                send(
                    &mut fork,
                    &mut renderer,
                    mouse::Cursor::Unavailable,
                    &[Event::Mouse(mouse::Event::CursorLeft),]
                )
                .is_empty()
            );
            assert!(
                capture(
                    &mut fork,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-fork-idle"),
                    mouse::Cursor::Unavailable,
                ) == idle
            );
            assert!(click(&mut fork, &mut renderer, Point::new(5.0, 5.0)).is_empty());
            assert!(key(&mut fork, &mut renderer, Key::Named(Named::Enter), None).is_empty());
            assert!(matches!(
                click(&mut fork, &mut renderer, local).as_slice(),
                [Message::Fork(chat_fork::Event::ForkSessionAt {
                    anchor,
                    target: ForkSessionTarget::Local
                })] if anchor.user_message_id == "second-turn"
            ));
            assert!(matches!(
                click(&mut fork, &mut renderer, worktree).as_slice(),
                [Message::Fork(chat_fork::Event::ForkSessionAt {
                    anchor,
                    target: ForkSessionTarget::NewWorktree
                })] if anchor.user_message_id == "second-turn"
            ));
            let cancel = label(&bounds, i18n.__("取消")).center();
            assert!(matches!(
                click(&mut fork, &mut renderer, cancel).as_slice(),
                [Message::Fork(chat_fork::Event::CloseForkSessionDialog)]
            ));

            let close = assert_control_alignment(
                with_chat_fork_dialog(&app, root()),
                &renderer,
                size,
                None,
            );
            assert!(matches!(
                click(&mut fork, &mut renderer, close).as_slice(),
                [Message::Fork(chat_fork::Event::CloseForkSessionDialog)]
            ));

            let mut reset = scene(with_chat_reset_dialog(&app, root()), &mut renderer, size);
            let bounds = geometry(&mut reset, &renderer);
            assert_fits(&bounds, size);
            capture(
                &mut reset,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-reset"),
                mouse::Cursor::Unavailable,
            );
            assert!(key(&mut reset, &mut renderer, Key::Named(Named::Enter), None).is_empty());
            assert!(click(&mut reset, &mut renderer, Point::new(5.0, 5.0)).is_empty());
            let confirm = label(&bounds, i18n.__("确认")).center();
            assert!(matches!(
                click(&mut reset, &mut renderer, confirm).as_slice(),
                [Message::Reset(chat_reset::Event::ResetSessionToMessage { msg_idx: 1 })]
            ));
            let cancel = label(&bounds, i18n.__("取消")).center();
            assert!(matches!(
                click(&mut reset, &mut renderer, cancel).as_slice(),
                [Message::Reset(chat_reset::Event::CloseResetMenu)]
            ));
        }
    }
}

const DRAFT: &str = "新草稿・新檔案・draft";
const VIEW_SIZE: Size<u32> = Size::new(420, 480);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_absent_modals_preserve_underlay() {
    let mut app = fixture();
    let mut renderer = renderer().await;
    app.dialog = None;
    app.reset = Some(3);
    for view in [with_chat_fork_dialog(&app, root()), with_chat_reset_dialog(&app, root())] {
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
    prepare_chat(&mut app);
    let mut renderer = renderer().await;
    for (view, kind) in [
        (with_chat_fork_dialog(&app, with_chat_reset_dialog(&app, root())), "fork"),
        (with_chat_reset_dialog(&app, root()), "reset"),
    ] {
        let mut ui = scene(view, &mut renderer, VIEW_SIZE);
        let messages = key(&mut ui, &mut renderer, Key::Named(Named::Escape), None);
        assert!(
            matches!(
                (kind, messages.as_slice()),
                ("fork", [Message::Fork(chat_fork::Event::CloseForkSessionDialog)])
                    | ("reset", [Message::Reset(chat_reset::Event::CloseResetMenu)])
            ),
            "Escape did not cancel {kind}"
        );
    }
}
