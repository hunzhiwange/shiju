//! 模型选择的桌面状态与消息映射。
use crate::app::{App, Message, message};
use desktop_chat_input::input_panel::acp_options::reasoning_effort_option;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use iced::Element;
use vw_api_acp_model::acp_model_source::AcpModelSource;
use desktop_chat_input::input_panel::model_views as shared;
use desktop_chat_input::input_panel::reasoning_message;
use desktop_chat_input::input_panel::reasoning_selector::{
    localized_reasoning_effort_label, reasoning_effort_control,
};
use super::reasoning_selector::session_reasoning;
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
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let runtime = app.current_session_runtime_ref();
    if runtime.is_some_and(|runtime| runtime.is_employee_chat()) {
        let label = SingleLineLabel::new(i18n.__("智能体原生模型").to_owned(), 13.0).shrink();
        return iced::widget::container(label)
            .center_y(super::styles::BOTTOM_BAR_ICON_BUTTON_SIZE)
            .padding([0, 10])
            .style(|theme| {
                let style = super::styles::composer_selector_button_style(
                    theme,
                    iced::widget::button::Status::Active,
                    false,
                );
                iced::widget::container::Style {
                    background: style.background,
                    border: style.border,
                    text_color: Some(style.text_color),
                    ..Default::default()
                }
            })
            .into();
    }
    let acp = runtime.filter(|runtime| runtime.acp_agent.is_some());
    let cli_local = acp.is_some_and(|runtime| runtime.acp_model_source == AcpModelSource::CliLocal);
    let reasoning_label = if let Some(runtime) = acp {
        app.session_state
            .active_session_id
            .as_ref()
            .and_then(|_| reasoning_effort_option(&runtime.acp_options_state.options))
            .and_then(|option| option.choices.iter().find(|choice| choice.id == option.current))
            .map(|choice| choice.name.as_str())
    } else {
        session_reasoning(app, model).map(|state| localized_reasoning_effort_label(i18n, state.current))
    };
    shared::model_toggle_button(
        i18n,
        &app.model_settings.providers,
        cli_local,
        auto_model,
        model,
        expanded,
        reasoning_label,
        emit,
    )
}
pub fn model_popover_content<'a>(app: &'a App, auto_model: bool, model: &str) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    if runtime.is_employee_chat() {
        return iced::widget::text(crate::app::i18n::I18n::new(app.general_settings.desktop_locale).__("智能体原生模型")).size(13).into();
    }
    let started = app.current_session_has_started();
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let can_select = shared::can_select_model(&super::acp_options::state(app, &runtime), started);
    let input = shared::ModelPopoverInput {
        settings: &app.model_settings, i18n,
        cli_local: runtime.acp_agent.is_some() && runtime.acp_model_source == AcpModelSource::CliLocal,
        started, can_select, is_acp: runtime.acp_agent.is_some(),
        input_model: runtime.acp_options_state.model_input_draft.as_deref().unwrap_or(model).to_owned(),
    };
    let reasoning = if input.is_acp {
        super::acp_options::reasoning_control(app, &runtime)
    } else {
        session_reasoning(app, model).map(|state| {
            reasoning_effort_control(
                i18n,
                app.session_state.active_session_id.as_deref(),
                model,
                &state.options,
                state.current,
            )
            .map(|reasoning_message::Message::Selected(value)| {
                Message::Chat(message::ChatMessage::ReasoningEffortSelected(value))
            })
        })
    };
    shared::model_popover_content(
        input,
        auto_model,
        model,
        super::acp_options::source_controls(app),
        reasoning,
        emit,
    )
}
