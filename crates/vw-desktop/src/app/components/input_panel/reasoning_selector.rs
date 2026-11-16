use crate::app::{App, Message, message};
use vw_api_chat_identity::ReasoningEffortDto;
use desktop_chat_input::input_panel::reasoning_selector as domain;
fn input(app: &App) -> domain::ReasoningInput<'_> {
    domain::ReasoningInput {
        providers: &app.model_settings.providers,
        model: app.current_session_runtime_ref().map(|r| r.model.as_str()).unwrap_or(&app.session_state.model),
        expanded: app.popover_visibility.reasoning_effort,
        i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        session_id: app.session_state.active_session_id.as_deref(),
    }
}
fn map_message(msg: desktop_chat_input::input_panel::reasoning_message::Message) -> Message {
    use desktop_chat_input::input_panel::reasoning_message::Message as Local;
    match msg {
        Local::Selected(value) => Message::Chat(message::ChatMessage::ReasoningEffortSelected(value)),
        Local::Toggle => Message::View(message::ViewMessage::ToggleReasoningEffortPopover),
        Local::Close => Message::View(message::ViewMessage::CloseReasoningEffortPopover),
    }
}
pub(crate) fn model_reasoning_effort_options(app: &App, model: &str) -> Vec<Option<ReasoningEffortDto>> {
    domain::model_reasoning_effort_options(&app.model_settings.providers, model)
}
pub(super) fn reasoning_effort_selector<'a>(app: &'a App, agent: Option<&str>, current: Option<ReasoningEffortDto>) -> Option<iced::Element<'a, Message>> {
    domain::reasoning_effort_selector(input(app), agent, current).map(|view| view.map(map_message))
}
