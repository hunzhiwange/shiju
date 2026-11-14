//! Transfer selection contains no implicit destinations, including same-name and Hermes entries.
use crate::app::{Message, components::system_settings_common::*, message::settings::prompts::{message, Event}};
use iced::{Element, widget::{button, checkbox, column, row, text}};
use vw_gateway_client::vw_api_types::agent_apps::AppId;
use crate::app::message::settings::prompts::transfer::Event as TransferEvent;

pub(crate) const TARGETS: &[AppId] = &[AppId::Claude, AppId::Codex, AppId::Gemini, AppId::OpenCode, AppId::Hermes, AppId::Pi, AppId::Mcode, AppId::Omp, AppId::GrokBuild, AppId::Kimi, AppId::Dsh, AppId::Agy];
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub entry: Option<String>,
    pub targets: Vec<AppId>,
    pub results: Vec<String>,
}
fn send(event: TransferEvent) -> Message { message(Event::Transfer(event)) }
pub(crate) fn overlay<'a>(state: &'a State, current: AppId, busy: bool, base: Element<'a, Message>) -> Element<'a, Message> {
    if state.entry.is_none() { return base; }
    let close = send(TransferEvent::Close);
    let mut fields = column![text("仅复制到勾选的应用，副本保持停用。所有目标默认不选；同名条目也不会自动覆盖。")].spacing(12);
    for &target in TARGETS.iter().filter(|&&id| id != current) {
        let mut control = checkbox(state.targets.contains(&target)).label(format!("{target:?}")).style(settings_checkbox_style);
        if !busy { control = control.on_toggle(move |selected| send(TransferEvent::Select(target, selected))); }
        fields = fields.push(control);
    }
    for result in &state.results { fields = fields.push(text(result)); }
    let footer = row![
        button(text("关闭")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe((!busy).then_some(close.clone())),
        button(text("复制所选目标")).padding([8, 12]).style(primary_action_btn_style).on_press_maybe((!busy && !state.targets.is_empty()).then(|| send(TransferEvent::Copy)))
    ].spacing(10);
    let header = settings_modal_header(text("复制到其他应用").size(20), settings_close_button(close.clone()));
    settings_modal_overlay(Some(base), close, settings_modal_layout(header, fields, Some(footer.into())).width(640).height(580))
}
