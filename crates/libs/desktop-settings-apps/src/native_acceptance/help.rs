//! Shared rendering assertions for settings help and direct host callers.
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::clipboard;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::{Id, Operation};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

pub const HELP_LOCALES: [(DesktopLocale, Theme, u32); 3] = [
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

pub async fn verify_help<Message: Clone + 'static>(
    view: Element<'_, Message>,
    name: &str,
    mode: &(DesktopLocale, Theme, u32),
    help: &[(&str, &str)],
    visible: &[&str],
    action: &str,
) -> Vec<Message> {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
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
