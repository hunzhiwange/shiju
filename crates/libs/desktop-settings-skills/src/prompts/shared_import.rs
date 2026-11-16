use desktop_ui_controls::system_settings_common::*;
#[derive(Clone, Debug)]
pub enum ImportEvent {
    Cancel,
    Enable(bool),
    Confirm,
}
// An inert share preview; only Confirm creates an SDK write.
use iced::{
    Element,
    widget::{button, checkbox, column, row, text},
};
use vw_api_app_operations::PromptSharePayload;

#[derive(Clone, Debug)]
pub struct Draft {
    pub payload: PromptSharePayload,
    pub enable_after_import: bool,
    pub error: Option<String>,
}
pub fn overlay<'a, M: Clone + 'static, C: Clone + 'a>(
    state: &'a super::State<C>,
    base: Element<'a, M>,
    message: fn(super::Event) -> M,
) -> Element<'a, M> {
    let message_event = move |event| message(super::Event::SharedImport(event));
    let (Some(draft), Some(scope)) = (&state.shared_import, &state.scope) else {
        return base;
    };
    let close = message_event(ImportEvent::Cancel);
    let mut body = column![
        text(format!(
            "目标应用：{:?} · 主机：{} · 项目：{}",
            scope.app_id, scope.target_host_id, scope.project_id.as_ref()
        )),
        text(format!("来源应用：{:?}", draft.payload.source_app)),
        text(&draft.payload.name).size(18),
        text(draft.payload.description.as_deref().unwrap_or("无描述")),
        text("正文预览"),
        text(&draft.payload.content),
    ]
    .spacing(12);
    let mut toggle =
        checkbox(draft.enable_after_import).label("导入后激活").style(settings_checkbox_style);
    if !state.mutating {
        toggle = toggle.on_toggle(move |value| message_event(ImportEvent::Enable(value)));
    }
    body = body.push(toggle).push(text(if draft.enable_after_import {
        "确认后创建条目并尝试替换目标应用当前提示词；版本冲突时保留停用副本，不自动重试。Pi 需 /reload，Hermes 在新会话生效。"
    } else { "确认后仅创建停用副本，不更改当前激活内容。取消不写入。" }));
    if let Some(error) = &draft.error {
        body = body.push(text(error));
    }
    let footer = row![
        button(text("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.mutating).then_some(close.clone())),
        button(text("确认导入"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe((!state.mutating).then(|| message_event(ImportEvent::Confirm)))
    ]
    .spacing(10);
    settings_modal_overlay(
        Some(base),
        close.clone(),
        settings_modal_layout(
            settings_modal_header(text("确认分享提示词").size(20), settings_close_button(close)),
            body,
            Some(footer.into()),
        )
        .width(700)
        .height(620),
    )
}
