#![cfg(not(target_arch = "wasm32"))]

use crate::autonomy_inputs::View;
use crate::autonomy_message::Message;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_settings_security::app_init::state::{AutonomySettingsState, AutonomySettingsTab};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Element, Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

// Use the same selectable help widget as the host, with only local message ports.
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            _ => panic!("unexpected help action"),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        panic!("help text must not publish chat selection")
    }
}
pub(crate) fn help<'a>(
    i18n: I18n,
    theme: Option<&Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    body: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            body,
            theme.map(|theme| theme.palette().text),
            i18n,
        ),
    )
}
pub(crate) fn inputs<'a>(
    state: &'a AutonomySettingsState,
    tools: &'a [String],
    locale: DesktopLocale,
    theme: &'a Theme,
) -> View<'a> {
    View {
        autonomy_settings: state,
        available_tools: tools,
        desktop_locale: locale,
        app_theme: theme,
        last_copied_code_hash: None,
        help_text: help,
    }
}
pub(crate) fn isolated(module: &str, test: &str) -> bool {
    const CHILD: &str = "SHIJU_HOST_SPLIT_2_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module.split_once("::").unwrap().1)])
        .env(CHILD, "1")
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

pub(crate) async fn renderer() -> Renderer {
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

pub(crate) type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

pub(crate) fn scene<'a>(
    view: Element<'a, Message>,
    renderer: &mut Renderer,
    size: Size<u32>,
) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

pub(crate) fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages.retain(|message| !matches!(message, Message::None));
    messages
}

pub(crate) fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    _name: &str,
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
    pixels
}
pub(crate) fn key(name: iced::keyboard::key::Named) -> Event {
    use iced::keyboard::{self, Key};
    let key = Key::Named(name);
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
    })
}

#[derive(Default)]
pub(crate) struct Geometry {
    pub(crate) focusable: Vec<iced::Rectangle>,
    pub(crate) labels: Vec<(String, iced::Rectangle)>,
    pub(crate) inputs: Vec<(String, iced::Rectangle)>,
}

impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }

    fn focusable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focusable.push(bounds);
    }

    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        state: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
}

impl Geometry {
    pub(crate) fn label(&self, value: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing label {value}: {:?}", self.labels))
            .1
    }

    pub(crate) fn input(&self, value: &str) -> iced::Rectangle {
        self.inputs
            .iter()
            .find(|(input, _)| input == value)
            .unwrap_or_else(|| panic!("missing fixture input {value}"))
            .1
    }
}

pub(crate) fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

pub(crate) fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) -> Vec<Message> {
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

pub(crate) fn assert_help_card(
    app: &View<'_>,
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    size: Size<u32>,
    title: &str,
    old_description: &str,
    body: &str,
) {
    let positions = geometry(ui, renderer);
    assert!(
        positions.labels.iter().all(|(label, _)| label != old_description),
        "{title}: static explanation is visible before opening help"
    );
    let bounds = positions.label(title);
    let prefix = format!("settings-help-{:?}-{}-{title}", app.desktop_locale, size.width);
    let theme = app.app_theme;
    let cursor = mouse::Cursor::Unavailable;
    let idle = capture(ui, renderer, theme, size, &format!("{prefix}-idle"), cursor);
    let icon = [
        iced::Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
        // The shared help label can wrap its icon below long labels.
        iced::Point::new(bounds.x + 11.0, bounds.y + bounds.height + 15.0),
    ]
    .into_iter()
    .find(|&icon| {
        let hover = mouse::Cursor::Available(icon);
        assert!(
            send(
                ui,
                renderer,
                hover,
                &[Event::Mouse(mouse::Event::CursorMoved { position: icon })]
            )
            .is_empty()
        );
        idle != capture(ui, renderer, theme, size, &format!("{prefix}-hover"), hover)
    })
    .unwrap_or_else(|| panic!("{title}: hover help did not render at {bounds:?}"));
    assert!(click(ui, renderer, icon).is_empty());
    let pinned = capture(ui, renderer, theme, size, &format!("{prefix}-pinned"), cursor);
    assert!(idle != pinned);
    assert_eq!(geometry(ui, renderer).inputs, positions.inputs);

    let side = if icon.x < size.width as f32 / 2.0 { 29.0 } else { -29.0 };
    let point = iced::Point::new(icon.x + side, icon.y + 5.0);
    assert!(
        send(
            ui,
            renderer,
            mouse::Cursor::Available(point),
            &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))]
        )
        .is_empty()
    );
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    let menu_height = 6.0 + 5.0 * item_height;
    let top = if point.y + menu_height > size.height as f32 {
        (point.y - menu_height).max(0.0)
    } else {
        point.y
    };
    let item = iced::Point::new(
        point.x.min(size.width as f32 - 140.0) + 24.0,
        top + 3.0 + item_height * 0.5,
    );
    let messages = click(ui, renderer, item);
    assert!(
        matches!(messages.as_slice(), [Message::CopyCode(value)] if value == body),
        "{title}: unexpected help text: {messages:?}"
    );

    let escape = key(iced::keyboard::key::Named::Escape);
    assert!(send(ui, renderer, cursor, &[escape]).is_empty());
    assert!(
        idle == capture(ui, renderer, theme, size, &format!("{prefix}-closed"), cursor),
        "{prefix}: Escape must restore the initial frame"
    );
}

pub(crate) const HELP_LOCALE_THEMES: [(DesktopLocale, Theme); 3] = [
    (DesktopLocale::SimplifiedChinese, Theme::Light),
    (DesktopLocale::TraditionalChinese, Theme::Dark),
    (DesktopLocale::English, Theme::Dark),
];
