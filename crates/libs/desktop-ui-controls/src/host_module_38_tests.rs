#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code, unused_imports)]
use crate::toast::{Toast, ToastKind, confirm_dialog};
#[derive(Debug, Clone)]
enum Message {
    Retry,
    Confirmed(String),
    Canceled,
}
type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

#[derive(Default)]
struct Feedback {
    desktop_locale: DesktopLocale,
    active_toast: Option<Toast>,
    next_id: usize,
    retry: bool,
}
impl Feedback {
    fn show_toast(&mut self, kind: ToastKind, text: String) {
        crate::toast::show_toast(&mut self.active_toast, &mut self.next_id, kind, text);
    }
    fn show_localized_toast(
        &mut self,
        kind: ToastKind,
        source: &str,
        args: Vec<desktop_i18n::format::Argument>,
    ) {
        let (message, localized) = crate::toast::localized_toast_message(source, args);
        self.show_toast(kind, message);
        self.active_toast.as_mut().unwrap().localized_message = Some(localized);
    }
    fn view(&self) -> Element<'static, Message> {
        crate::toast::view(
            self.active_toast.as_ref(),
            self.desktop_locale,
            self.retry.then_some(Message::Retry),
        )
    }
}
fn fixture() -> Feedback {
    Feedback::default()
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_33_SCREENSHOTS") {
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

fn find_layout<'a>(
    layout: Layout<'a>,
    matches: impl Fn(Rectangle) -> bool + Copy,
) -> Option<Layout<'a>> {
    if matches(layout.bounds()) {
        return Some(layout);
    }
    layout.children().find_map(|child| find_layout(child, matches))
}

fn assert_centered_controls(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    title: Rectangle,
    input: Rectangle,
    toast: Rectangle,
) -> Point {
    let mut tree = Tree::new(view.as_widget());
    let limits =
        layout::Limits::new(Size::ZERO, Size::new(NARROW.width as f32, NARROW.height as f32));
    let node = view.as_widget_mut().layout(&mut tree, renderer, &limits);
    let root = Layout::new(&node);
    let close = find_layout(root, |bounds| {
        bounds.width == 28.0
            && bounds.height == 28.0
            && (bounds.center_y() - title.center_y()).abs() < 0.5
            && bounds.x > title.x
    })
    .expect("close button aligned with the title");
    let icon = find_layout(close, |bounds| bounds.width == 12.0 && bounds.height == 12.0)
        .expect("close icon");
    assert!((close.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    let field = find_layout(root, |bounds| bounds == input).expect("custom input");
    let glyphs = field.children().next().expect("input text");
    assert!((input.center_y() - glyphs.bounds().center_y()).abs() < 0.5);
    let badge = find_layout(root, |bounds| {
        bounds.width == 34.0
            && bounds.height == 34.0
            && bounds.x < toast.x
            && bounds.y <= toast.center_y()
            && bounds.y + bounds.height >= toast.center_y()
    })
    .expect("toast badge");
    let icon = find_layout(badge, |bounds| bounds.width == 14.0 && bounds.height == 14.0)
        .expect("toast icon");
    assert!((badge.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    close.bounds().center()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toast_text_wraps_as_needed_and_leaves_room_for_retry() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let mut renderer = renderer().await;
    let _ = app.show_localized_toast(ToastKind::Success, "已删除", vec![]);
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let short = label(&geometry(&mut ui, &renderer), "操作已完成 · 已删除");
    assert!(short.height < 20.0, "{short:?}");
    for (theme, name) in [(Theme::Light, "light-short-toast"), (Theme::Dark, "dark-short-toast")] {
        capture(&mut ui, &mut renderer, &theme, NARROW, name, mouse::Cursor::Unavailable);
    }
    drop(ui);

    let path = "/fixture/".to_owned() + &"filename".repeat(16);
    let _ = app.show_toast(ToastKind::Error, path.clone());
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let long = label(&geometry(&mut ui, &renderer), &format!("操作失败 · {path}"));
    assert!(long.height > short.height && long.x + long.width <= NARROW.width as f32, "{long:?}");
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        NARROW,
        "dark-long-toast",
        mouse::Cursor::Unavailable,
    );
    drop(ui);

    app.retry = true;
    let _ = app.show_toast(ToastKind::Warning, format!("本地缓存待同步：{path}"));
    let mut ui = scene(app.view(), &mut renderer, NARROW);
    let retry = label(&geometry(&mut ui, &renderer), "重试");
    assert!(retry.width > 0.0 && retry.x + retry.width <= NARROW.width as f32, "{retry:?}");
    assert!(matches!(click(&mut ui, &mut renderer, retry.center()).as_slice(), [Message::Retry]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_33_confirmation_preserves_messages() {
    const PATH: &str = "/fixture/測試";
    let mut renderer = renderer().await;
    for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let dialog = confirm_dialog(
            "移除項目 · Remove fixture",
            "移除虚构记录。 Remove the fixture record. ".repeat(24),
            "Remove fixture",
            "Cancel",
            Message::Confirmed(PATH.into()),
            Message::Canceled,
        );
        let mut ui = scene(dialog, &mut renderer, NARROW);
        let bounds = geometry(&mut ui, &renderer);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            NARROW,
            &format!("{mode}-confirm"),
            mouse::Cursor::Unavailable,
        );
        let confirm = label(&bounds, "Remove fixture");
        let cancel = label(&bounds, "Cancel");
        let (viewport, _) = bounds.scroll[0];
        assert!(
            send(
                &mut ui,
                &mut renderer,
                mouse::Cursor::Available(viewport.center()),
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: -10.0 },
                })],
            )
            .is_empty()
        );
        let after = geometry(&mut ui, &renderer);
        assert!(after.scroll[0].1.y > 0.0);
        assert_eq!(confirm, label(&after, "Remove fixture"));
        assert_eq!(cancel, label(&after, "Cancel"));
        for key_name in [Named::Enter, Named::Escape] {
            assert!(key(&mut ui, &mut renderer, Key::Named(key_name), None).is_empty());
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, confirm.center()).as_slice(),
            [Message::Confirmed(path)] if path == PATH
        ));
        let messages = click(&mut ui, &mut renderer, cancel.center());
        assert!(matches!(messages.as_slice(), [Message::Canceled]));
    }
}

const NARROW: Size<u32> = Size::new(420, 480);
