use crate::app::{App, Message, message};
use vw_api_chat_identity::ReasoningEffortDto;
use desktop_chat_input::input_panel::reasoning_selector as domain;
pub(crate) use domain::{reasoning_effort_label, localized_reasoning_effort_label, reasoning_effort_options};
fn input(app: &App) -> domain::ReasoningInput<'_> {
    domain::ReasoningInput {
        providers: &app.model_settings.providers,
        model: app.current_session_runtime_ref().map(|r| r.model.as_str()).unwrap_or(&app.model),
        expanded: app.popover_visibility.reasoning_effort,
        i18n: crate::app::i18n::I18n::new(app.desktop_locale),
        session_id: app.active_session_id.as_deref(),
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

#[cfg(test)]
pub(super) fn reasoning_effort_popover<'a>(app: &'a App, model: &str, options: &[Option<ReasoningEffortDto>], current: Option<ReasoningEffortDto>) -> iced::Element<'a, Message> {
    domain::reasoning_effort_popover(input(app), model, options, current).map(map_message)
}
