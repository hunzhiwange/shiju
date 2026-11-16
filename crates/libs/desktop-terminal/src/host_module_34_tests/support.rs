use crate::message::{Message, TerminalMessage};
use crate::{Shell, TerminalState, TerminalTheme, ViewInput};
pub struct Routing {
    pub terminal: TerminalState<Shell>,
    pub focused: bool,
    pub cursor_y: f32,
    pub(super) persistence: crate::updates::config::ConfigPersistence,
}
impl Routing {
    pub fn new() -> Self {
        Self {
            terminal: TerminalState::blank_with_settings(
                true,
                Shell::Bash,
                TerminalTheme::System,
                "monospace".into(),
                15.0,
                220.0,
            ),
            focused: false,
            cursor_y: 0.0,
            persistence: Default::default(),
        }
    }
}
pub fn update(input: &mut Routing, message: TerminalMessage) -> iced::Task<TerminalMessage> {
    crate::updates::update(
        &mut input.terminal,
        &mut input.persistence,
        &iced::Theme::Dark,
        None,
        Default::default(),
        input.cursor_y,
        &mut input.focused,
        &mut Vec::new(),
        message,
    )
}
pub fn view(input: &Routing) -> iced::Element<'_, Message> {
    view_with_locale(input, Default::default())
}
pub fn view_with_locale(
    input: &Routing,
    locale: config_ui::DesktopLocale,
) -> iced::Element<'_, Message> {
    crate::terminal_panel::view(ViewInput {
        tabs: &input.terminal.tabs,
        active_id: input.terminal.active_id,
        is_visible: input.terminal.is_visible,
        height: input.terminal.height,
        tab_context_menu_id: input.terminal.tab_context_menu_id,
        tab_context_menu_pos: input.terminal.tab_context_menu_pos,
        desktop_locale: locale,
        focused: input.focused,
        #[cfg(not(target_arch = "wasm32"))]
        panes: &input.terminal.panes,
        #[cfg(not(target_arch = "wasm32"))]
        content_menu: input.terminal.content_menu,
        #[cfg(not(target_arch = "wasm32"))]
        is_fullscreen: input.terminal.is_fullscreen,
    })
}

pub fn dispatch(input: &mut Routing, message: Message) {
    match message {
        Message::Terminal(message) => {
            let _ = update(input, message);
        }
        Message::Batch(messages) => {
            for message in messages {
                dispatch(input, message);
            }
        }
        Message::Hide => input.terminal.set_visible(false),
    }
}
