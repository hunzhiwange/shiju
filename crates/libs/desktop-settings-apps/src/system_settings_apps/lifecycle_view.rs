use vw_api_app_operations::{AppActionPreviewRequest, AppLifecycleAction, AppOperationPhase};
use vw_api_app_identity::AppInstallationSource;

use desktop_ui_controls::system_settings_common::*;
use super::{Action, lifecycle::LifecycleState};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
pub fn action_button<'a, M: Clone + 'static>(
    label: &'a str,
    request: AppActionPreviewRequest,
    enabled: bool,
    event: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    button(text(label))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(event(Action::Preview(request))))
        .into()
}
pub fn progress<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n, state: &'a LifecycleState, event: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
    let mut content = column![].spacing(8);
    if let Some(batch) = &state.batch {
        content = content.push(text(i18n.sprintf("批量升级：成功 %u · 失败 %s · 取消 %s · %s %s", &[(batch.completed).into(), format!("{}", batch.failed).into(), format!("{}", batch.cancelled).into(), (if batch.stopped { i18n.__("跳过 / 未执行") } else { i18n.__("等待") }).to_string().into(), format!("{}", batch.pending()).into()])).size(13));
        if !batch.stopped {
            content = content.push(button(text(i18n.__("停止批量升级")))
                .padding([8, 16]).style(rounded_action_btn_style)
                .on_press(event(Action::LifecycleCancel)));
        }
    }
    if let Some(request) = &state.request {
        content = content.push(
            text(i18n.sprintf("应用：%s · %s", &[format!("{:?}", request.app_id).into(), (match request.action {
                    AppLifecycleAction::Install => i18n.__("安装"),
                    AppLifecycleAction::Update => i18n.__("升级"),
                    AppLifecycleAction::Uninstall => i18n.__("卸载"),
                }).to_string().into()]))
            .size(13),
        );
    }
    if !state.target_label.is_empty() {
        content = content.push(text(&state.target_label).size(13));
    }
    if let Some(operation) = &state.operation {
        content = content.push(
            text(i18n.sprintf("操作 %s\n%s · %s", &[operation.operation_id.0.to_string().into(), format!("{:?}", operation.phase).into(), format!("{:?}", operation.status).into()]))
            .size(13),
        );
        if operation.phase != AppOperationPhase::Terminal {
            content = content.push(
                row![
                    button(text(i18n.__("刷新操作 / 恢复连接")))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press(event(Action::LifecycleRefresh)),
                    button(text(if state.cancel_requested { i18n.__("已请求取消") } else { i18n.__("取消操作") }))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!state.cancel_requested).then_some(event(Action::LifecycleCancel))),
                ]
                .spacing(10),
            );
        }
    } else if state.busy {
        content = content.push(text(i18n.__("正在预检或提交，请稍候…")));
    }
    if state.cancel_requested {
        content = content.push(
            text(i18n.__("正在请求取消或等待服务端确认；任务仍可能完成，以最终状态为准。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    content.into()
}
pub fn overlay<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &'a LifecycleState,
    _host: &'a str,
    base: Element<'a, M>,
    guidance: &'a str,
    event: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    if let Some(batch) = state.batch.as_ref().filter(|b| !b.confirmed && !b.stopped) {
        let close = event(Action::LifecycleClose);
        let header = settings_modal_header(
            settings_page_heading(i18n.__("确认全部升级"), i18n.__("逐项预检并串行执行；失败、目标变化或取消时停止。")),
            settings_close_button(close.clone()),
        );
        let mut body = column![].spacing(12);
        for item in &batch.items {
            body = body.push(text(i18n.sprintf("%s · %s\n实例：%s · %s", &[format!("{:?}", item.app_id).into(), item.path.to_string().into(), item.id.0.to_string().into(), format!("{:?}", item.source).into()]))
                .size(13).wrapping(iced::widget::text::Wrapping::WordOrGlyph));
        }
        let footer = row![
            Space::new().width(Length::Fill),
            button(text(i18n.__("取消"))).padding([8, 16]).style(rounded_action_btn_style).on_press(close.clone()),
            button(text(i18n.__("确认全部升级"))).padding([8, 16]).style(primary_action_btn_style)
                .on_press(event(Action::LifecycleConfirm)),
        ].spacing(10);
        let card = settings_modal_layout(header, body, Some(footer.into()))
            .width(Length::Fill).max_width(520.0).max_height(484.0);
        return settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill));
    }
    let Some(preview) = &state.preview else {
        return base;
    };
    let close = event(Action::LifecycleClose);
    let (title, guidance, confirm_label) = match preview.request.action {
        AppLifecycleAction::Install => {
            (i18n.__("确认安装"), i18n.__("确认后由网关重新检查目标与 Node 依赖；排队不表示安装成功。"), i18n.__("确认提交"))
        }
        AppLifecycleAction::Update => {
            (i18n.__("确认升级"), i18n.__("确认后由网关重新检查目标与 Node 依赖；排队不表示升级成功。"), i18n.__("确认提交"))
        }
        AppLifecycleAction::Uninstall => {
            (i18n.__("确认卸载所选实例"), guidance, i18n.__("确认卸载"))
        }
    };
    let uninstalling = preview.request.action == AppLifecycleAction::Uninstall;
    let header = settings_modal_header(
        settings_page_heading(title, i18n.__("请核对所选安装实例。")),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(i18n.sprintf("来源：%s", &[format!("{:?}", preview.source).into()])).size(13).style(settings_muted_text_style),
        text(&state.target_label).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        settings_divider(),
        text(&preview.summary).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        text(i18n.__(guidance)).size(13).style(settings_muted_text_style),
    ]
    .spacing(12);
    if uninstalling {
        body = body.push(
            text(i18n.__("计划或路径变化时拒绝执行；请重新预检并确认，不会自动重试删除。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(
            text(if state.busy { i18n.__("处理中…") } else { confirm_label })
                .size(SETTINGS_CONTROL_TEXT_SIZE)
        )
        .padding([8, 16])
        .style(if uninstalling { danger_action_btn_style } else { primary_action_btn_style })
        .on_press_maybe(
            (!state.busy
                && !state.uncertain
                && state.request.as_ref() == Some(&preview.request)
                && preview.source != AppInstallationSource::Unknown)
                .then_some(event(Action::LifecycleConfirm)),
        ),
    ]
    .align_y(Alignment::Center)
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(520.0)
        .max_height(484.0);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
