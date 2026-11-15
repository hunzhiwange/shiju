use desktop_ui_controls::system_settings_common::*;
#[derive(Clone, Debug)]
pub enum TransferEvent {
    Open(String),
    Close,
    Select(vw_api_app_identity::AppId, bool),
    Copy,
    Body(String),
    Path,
    Reveal,
    ImportHost,
    ImportClient,
}
// Transfer selection contains no implicit destinations, including same-name and Hermes entries.
use iced::{
    Element,
    widget::{button, checkbox, column, row, text},
};
use vw_api_app_identity::AppId;

pub const TARGETS: &[AppId] = &[
    AppId::Claude,
    AppId::Codex,
    AppId::Gemini,
    AppId::OpenCode,
    AppId::Hermes,
    AppId::Pi,
    AppId::Mcode,
    AppId::Omp,
    AppId::GrokBuild,
    AppId::Kimi,
    AppId::Dsh,
    AppId::Agy,
];
#[derive(Clone, Debug, Default)]
pub struct State {
    pub entry: Option<String>,
    pub targets: Vec<AppId>,
    pub results: Vec<String>,
}
pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    current: AppId,
    busy: bool,
    base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    let message_event = move |event| message(super::Event::Transfer(event));
    if state.entry.is_none() {
        return base;
    }
    let close = message_event(TransferEvent::Close);
    let mut fields = column![text(
        "仅复制到勾选的应用，副本保持停用。所有目标默认不选；同名条目也不会自动覆盖。"
    )]
    .spacing(12);
    for &target in TARGETS.iter().filter(|&&id| id != current) {
        let mut control = checkbox(state.targets.contains(&target))
            .label(format!("{target:?}"))
            .style(settings_checkbox_style);
        if !busy {
            control = control
                .on_toggle(move |selected| message_event(TransferEvent::Select(target, selected)));
        }
        fields = fields.push(control);
    }
    for result in &state.results {
        fields = fields.push(text(result));
    }
    let footer = row![
        button(text("关闭"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then_some(close.clone())),
        button(text("复制所选目标"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!busy && !state.targets.is_empty()).then(|| message_event(TransferEvent::Copy))
            )
    ]
    .spacing(10);
    let header = settings_modal_header(
        text("复制到其他应用").size(20),
        settings_close_button(close.clone()),
    );
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, fields, Some(footer.into())).width(640).height(580),
    )
}
