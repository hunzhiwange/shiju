use super::{LaunchChoice, LaunchMessage, event};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, state::AppsCatalogState};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::agent_apps::{LaunchIdentityKind, LaunchSessionPhase};

pub(crate) fn overlay<'a>(
    catalog: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &catalog.launch;
    let Some(request) = &state.request else {
        return base;
    };
    let close = event(LaunchMessage::Close);
    let editable = !state.busy && !state.accepted;
    let mut body = column![
        text(format!("执行主机：{}", catalog.host)).size(14),
        text(format!("应用：{:?} · 实例：{}", request.app_id, request.installation_id.0))
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    if state.busy {
        body = body.push(text("等待执行主机确认；不会自动重试…").size(13));
    }
    if state.accepted {
        body = body.push(
            text(match state.phase {
                Some(LaunchSessionPhase::Completed) => "CLI 已退出，执行端回写与清理完成",
                Some(LaunchSessionPhase::Failed) => "CLI 会话失败；请检查执行主机与账号状态",
                Some(LaunchSessionPhase::Starting | LaunchSessionPhase::Running) => {
                    "CLI 会话运行中；关闭终端将终止并回收进程"
                }
                None => "终端已接受启动，尚未确认 CLI 完成",
            })
            .size(13),
        );
    }
    if state.identities_loading {
        body = body.push(text("正在读取所选主机可用身份…").size(13));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }

    let mut identities = column![settings_field_label("启动身份")].spacing(8);
    if state.identities_ready {
        let choices = std::iter::once(LaunchChoice::ExistingConfiguration)
            .chain(state.identities.iter().cloned().map(LaunchChoice::Identity));
        identities = identities.push(
            settings_select(choices, state.selected.clone(), |v| event(LaunchMessage::Select(v)))
                .placeholder("选择主机现有配置、供应商或官方账号")
                .disabled(!editable)
                .width(Length::Fill),
        );
        for (kind, label) in [
            (LaunchIdentityKind::Provider, "暂无可用供应商"),
            (LaunchIdentityKind::OfficialAccount, "暂无可用官方账号"),
        ] {
            if !state.identities.iter().any(|i| i.kind == kind) {
                identities = identities.push(text(label).size(12).style(settings_muted_text_style));
            }
        }
    } else if let Some(selected) = &state.selected {
        identities = identities.push(text(selected.to_string()).size(13));
    }

    let mut args =
        column![settings_group_heading("附加参数", "每项一个参数，原样传递；不解释 shell。",)]
            .spacing(8);
    for (index, value) in state.args.iter().enumerate() {
        args = args.push(
            row![
                settings_text_input("参数内容", value).on_input_maybe(
                    editable.then_some(move |v| event(LaunchMessage::Args(index, v)))
                ),
                button(text("移除").size(SETTINGS_CONTROL_TEXT_SIZE))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(editable.then_some(event(LaunchMessage::RemoveArg(index)))),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        );
    }
    args = args.push(
        button(text("添加参数").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(event(LaunchMessage::AddArg))),
    );
    let cwd = column![
        settings_field_label("工作目录（由执行主机验证）"),
        settings_text_input("所选主机上的绝对目录", &request.cwd)
            .on_input_maybe(editable.then_some(|v| event(LaunchMessage::Cwd(v)))),
    ]
    .spacing(6);
    let body = body.push(identities).push(cwd).push(settings_divider()).push(args);

    let header = settings_modal_header(
        settings_page_heading("终端启动", "默认保留权限检查；此表单不提供跳过权限。"),
        settings_close_button(close.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text("关闭").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(close.clone())),
        button(
            text(if catalog.host == vw_shared::local_apps::HOST_ID {
                "在本机启动"
            } else {
                "在所选主机启动"
            })
            .size(SETTINGS_CONTROL_TEXT_SIZE),
        )
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(state.blocked_reason().is_none().then_some(event(LaunchMessage::Submit))),
    ]
    .align_y(Alignment::Center)
    .spacing(10);
    let footer = column![
        text(state.blocked_reason().unwrap_or("准备就绪，点击启动将在所选主机打开终端。"))
            .size(12)
            .style(settings_muted_text_style),
        actions,
    ]
    .spacing(8);
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(footer.into()),
        Some("apps-launch-body"),
    )
    .width(Length::Fill)
    .max_width(520.0)
    .max_height(560.0);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
