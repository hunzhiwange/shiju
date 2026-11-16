//! 模型选择的桌面状态与消息映射。
use crate::app::{App, Message, message};
use iced::Element;
use vw_api_acp_model::acp_model_source::AcpModelSource;
use desktop_chat_input::input_panel::model_views as shared;
fn emit(action: shared::Action) -> Message {
    match action {
        shared::Action::ToggleModelPopover => Message::View(message::ViewMessage::ToggleModelPopover),
        shared::Action::ModelQueryChanged(value) => Message::Settings(message::SettingsMessage::ModelQueryChanged(value)),
        shared::Action::ToggleInputModelProvider(value) => Message::View(message::ViewMessage::ToggleInputModelProvider(value)),
        shared::Action::ModelSelected(value) => Message::Chat(message::ChatMessage::ModelSelected(value)),
        shared::Action::OpenSystemSettingsModelDetail(provider, model) => Message::View(message::ViewMessage::OpenSystemSettingsModelDetail(provider, model)),
        shared::Action::AutoModelToggled(value) => Message::Chat(message::ChatMessage::AutoModelToggled(value)),
        shared::Action::AcpModelInputDraftChanged(value) => Message::Chat(message::ChatMessage::AcpModelInputDraftChanged(value)),
        shared::Action::ModelInputChanged(value) => Message::Chat(message::ChatMessage::ModelInputChanged(value)),
        shared::Action::ManageModels => Message::View(message::ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Models)),
        shared::Action::ConnectProvider => Message::View(message::ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Providers)),
    }
}
pub fn model_toggle_button(app: &App, auto_model: bool, model: &str, expanded: bool) -> Element<'static, Message> {
    let cli_local = app.current_session_runtime_ref().is_some_and(|r| r.acp_agent.is_some() && r.acp_model_source == AcpModelSource::CliLocal);
    shared::model_toggle_button(crate::app::i18n::I18n::new(app.desktop_locale), &app.model_settings.providers, cli_local, auto_model, model, expanded, emit)
}
pub fn model_popover_content<'a>(app: &'a App, auto_model: bool, model: &str) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    let started = app.current_session_has_started();
    let can_select = shared::can_select_model(&super::acp_options::state(app, &runtime), started);
    let input = shared::ModelPopoverInput {
        settings: &app.model_settings, i18n: crate::app::i18n::I18n::new(app.desktop_locale),
        cli_local: runtime.acp_agent.is_some() && runtime.acp_model_source == AcpModelSource::CliLocal,
        started, can_select, is_acp: runtime.acp_agent.is_some(),
        input_model: runtime.acp_options_state.model_input_draft.as_deref().unwrap_or(model).to_owned(),
    };
    shared::model_popover_content(input, auto_model, model, super::acp_options::source_controls(app), emit)
}
