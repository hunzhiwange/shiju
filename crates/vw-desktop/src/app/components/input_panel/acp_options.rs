//! ACP 输入投影与桌面消息接线。
use crate::app::{App, Message, i18n::I18n, message::ChatMessage};
use iced::Element;
use desktop_chat_input::input_panel::acp_options as shared;
pub(crate) use shared::OPTIONS_SCROLL;
fn emit(action: shared::Action) -> Message {
    Message::Chat(match action {
        shared::Action::AcpOptionsOpen => ChatMessage::AcpOptionsOpen,
        shared::Action::AcpOptionsClose => ChatMessage::AcpOptionsClose,
        shared::Action::AcpOptionSelected(selection) => ChatMessage::AcpOptionSelected(selection),
        shared::Action::AcpModelSourceSelected(source) => ChatMessage::AcpModelSourceSelected(source),
    })
}
fn state<'a>(app: &App, runtime: &'a crate::app::state::SessionRuntimeState) -> shared::OptionsState<'a> {
    shared::OptionsState {
        i18n: I18n::new(app.desktop_locale), acp_agent: runtime.acp_agent.as_deref(),
        acp_options: &runtime.acp_options_state.options, acp_model_source: runtime.acp_model_source,
        is_requesting: runtime.is_requesting, acp_options_pending: runtime.acp_options_state.pending,
        source_pending: runtime.acp_options_state.source_pending.is_some(), auto_model: runtime.auto_model,
        model: &runtime.model, full_access_enabled: runtime.full_access_enabled,
        acp_rebuild_required: runtime.acp_rebuild_required,
    }
}
pub(super) fn entry<'a>(app: &'a App, runtime: Option<&crate::app::state::SessionRuntimeState>) -> Option<Element<'a, Message>> {
    let runtime = runtime.filter(|runtime| runtime.acp_agent.is_some() && runtime.draft.is_none())?;
    shared::entry(I18n::new(app.desktop_locale), true,
        !runtime.is_requesting && !runtime.acp_options_state.pending && runtime.acp_options_state.source_pending.is_none(), emit)
}
pub(super) fn source_controls(app: &App) -> Element<'_, Message> {
    let runtime = app.current_session_runtime();
    source_controls_for(app, &runtime)
}
fn source_controls_for<'a>(app: &'a App, runtime: &crate::app::state::SessionRuntimeState) -> Element<'a, Message> {
    let title = "模型来源";
    let description = "客户端模型：使用当前供应商与模型，无需重复填写密钥\n智能体原生模型：使用执行主机上的配置与登录状态";
    let help = crate::app::components::system_settings_common::settings_hover_help_label(
        I18n::new(app.desktop_locale), &app.app_theme, title,
        crate::app::components::system_settings_common::settings_field_label(title), description,
    ).wrap().into();
    shared::source_controls(&state(app, runtime), app.current_session_has_started(), help, emit)
}
pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(runtime) = app.current_session_runtime_ref().filter(|r| r.acp_options_state.open && r.acp_agent.is_some()) else { return base; };
    let input = state(app, runtime);
    let can_change = super::send_controls::has_permission_context(app) && !runtime.is_requesting
        && !runtime.full_access_saving && !runtime.acp_options_state.pending && runtime.acp_options_state.source_pending.is_none();
    let permission = super::send_controls::acp_permission_modes(runtime.full_access_enabled, can_change);
    let access = shared::access_status(&input, permission);
    let source = source_controls_for(app, runtime);
    shared::with_modal(&input, input.i18n, app.current_session_has_started(), base, access, source, emit)
}
#[cfg(test)]
#[path = "acp_options_ui_refresh_42_tests.rs"]
pub(super) mod ui_refresh_42_tests;
