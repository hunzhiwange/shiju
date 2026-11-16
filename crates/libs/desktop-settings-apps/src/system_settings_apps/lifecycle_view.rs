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
pub fn progress<'a, M: Clone + 'static>(state: &'a LifecycleState, event: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
    let mut content = column![].spacing(8);
    if let Some(request) = &state.request {
        content = content.push(
            text(format!(
                "应用：{:?} · {}",
                request.app_id,
                match request.action {
                    AppLifecycleAction::Install => "安装",
                    AppLifecycleAction::Update => "升级",
                    AppLifecycleAction::Uninstall => "卸载",
                }
            ))
            .size(13),
        );
    }
    if !state.target_label.is_empty() {
        content = content.push(text(&state.target_label).size(13));
    }
    if let Some(operation) = &state.operation {
        content = content.push(
            text(format!(
                "操作 {}\n{:?} · {:?}",
                operation.operation_id.0, operation.phase, operation.status
            ))
            .size(13),
        );
        if operation.phase != AppOperationPhase::Terminal {
            content = content.push(
                row![
                    button(text("刷新操作 / 恢复连接"))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press(event(Action::LifecycleRefresh)),
                    button(text(if state.cancel_requested { "已请求取消" } else { "取消操作" }))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!state.cancel_requested).then_some(event(Action::LifecycleCancel))),
                ]
                .spacing(10),
            );
        }
    } else if state.busy {
        content = content.push(text("正在预检或提交，请稍候…"));
    }
    if state.cancel_requested {
        content = content.push(
            text("正在请求取消或等待服务端确认；任务仍可能完成，以最终状态为准。")
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    content.into()
}
pub fn overlay<'a, M: Clone + 'static>(
    state: &'a LifecycleState,
    host: &'a str,
    base: Element<'a, M>,
    guidance: &'a str,
    event: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    let Some(preview) = &state.preview else {
        return base;
    };
    let close = event(Action::LifecycleClose);
    let (title, guidance, confirm_label) = match preview.request.action {
        AppLifecycleAction::Install => {
            ("确认安装", "确认后由网关重新检查目标与 Node 依赖；排队不表示安装成功。", "确认提交")
        }
        AppLifecycleAction::Update => {
            ("确认升级", "确认后由网关重新检查目标与 Node 依赖；排队不表示升级成功。", "确认提交")
        }
        AppLifecycleAction::Uninstall => {
            ("确认卸载所选实例", guidance, "确认卸载")
        }
    };
    let uninstalling = preview.request.action == AppLifecycleAction::Uninstall;
    let header = settings_modal_header(
        settings_page_heading(title, "请核对执行主机和预检目标。"),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(format!("执行主机：{host}")).size(14),
        text(format!("来源：{:?}", preview.source)).size(13).style(settings_muted_text_style),
        text(&state.target_label).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        settings_divider(),
        text(&preview.summary).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        text(guidance).size(13).style(settings_muted_text_style),
    ]
    .spacing(12);
    if uninstalling {
        body = body.push(
            text("计划或路径变化时拒绝执行；请重新预检并确认，不会自动重试删除。")
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(
            text(if state.busy { "处理中…" } else { confirm_label })
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
