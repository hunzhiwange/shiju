use super::{LaunchChoice, LaunchMessage, event};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, state::AppsCatalogState};
use iced::{
    Element, Length,
    widget::{button, column, container, pick_list, row, scrollable, text, text_input},
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
    let mut status = column![].spacing(12);
    if state.busy {
        status = status.push(text("等待执行主机确认；不会自动重试…"));
    }
    if state.accepted {
        status = status.push(
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
        status = status.push(text("正在读取所选主机可用身份…"));
    }
    let choices: Vec<_> = std::iter::once(LaunchChoice::ExistingConfiguration)
        .chain(state.identities.iter().cloned().map(LaunchChoice::Identity))
        .collect();
    let mut identities = column![text("启动身份").size(13)].spacing(8);
    if state.identities_ready && !state.busy && !state.accepted {
        identities = identities.push(
            pick_list(choices, state.selected.clone(), |v| event(LaunchMessage::Select(v)))
                .placeholder("选择主机现有配置、供应商或官方账号")
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .text_size(SETTINGS_CONTROL_TEXT_SIZE),
        );
    } else if let Some(selected) = &state.selected {
        identities = identities.push(text(selected.to_string()).size(13));
    }
    for (kind, label) in [
        (LaunchIdentityKind::Provider, "供应商"),
        (LaunchIdentityKind::OfficialAccount, "官方账号"),
    ] {
        if state.identities_ready && !state.identities.iter().any(|i| i.kind == kind) {
            identities = identities.push(
                button(text(format!("{label}（暂无可用记录）")))
                    .padding([8, 12])
                    .style(rounded_action_btn_style),
            );
        }
    }
    if let Some(error) = &state.error {
        status = status.push(settings_error_banner(error.clone()));
    }
    let editable = !state.busy && !state.accepted;
    let mut args =
        column![text("附加参数（每项一个参数，原样传递；不解释 shell）").size(13)].spacing(8);
    for (index, value) in state.args.iter().enumerate() {
        args = args.push(
            row![
                text_input("参数内容", value)
                    .on_input_maybe(
                        editable.then_some(move |v| event(LaunchMessage::Args(index, v)))
                    )
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style),
                button(text("移除"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(editable.then_some(event(LaunchMessage::RemoveArg(index)))),
            ]
            .spacing(8),
        );
    }
    args = args.push(
        button(text("添加参数"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(event(LaunchMessage::AddArg))),
    );
    let body = column![
        status,
        text(format!("执行主机：{}", catalog.host)).size(14),
        text(format!("应用：{:?} · 实例：{}", request.app_id, request.installation_id.0)).size(13),
        identities,
        text("工作目录（由执行主机验证）").size(13),
        text_input("所选主机上的绝对目录", &request.cwd)
            .on_input_maybe(
                (!state.busy && !state.accepted).then_some(|v| event(LaunchMessage::Cwd(v)))
            )
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style),
        args,
        text("默认保留权限检查；此表单不提供跳过权限。").size(12),
    ]
    .spacing(12);
    let card = settings_modal_card(
        column![
            row![
                container(text("终端启动").size(22)).width(Length::Fill),
                settings_close_button(close.clone())
            ],
            scrollable(container(body).padding([0, 7]).width(Length::Fill))
                .id("apps-launch-body")
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            text(state.blocked_reason().unwrap_or("准备就绪，点击启动将在所选主机打开终端。"))
                .size(12)
                .style(settings_muted_text_style),
            row![
                button(text("关闭"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then_some(close.clone())),
                button(text(if catalog.host == vw_shared::local_apps::HOST_ID {
                    "在本机启动"
                } else {
                    "在所选主机启动"
                }))
                .padding([8, 16])
                .style(|theme, status| if status == iced::widget::button::Status::Disabled {
                    rounded_action_btn_style(theme, status)
                } else {
                    primary_action_btn_style(theme, status)
                })
                .on_press_maybe(
                    state.blocked_reason().is_none().then_some(event(LaunchMessage::Submit))
                ),
            ]
            .spacing(10),
        ]
        .spacing(16)
        .height(Length::Fill),
    )
    .width(Length::Fixed(520.))
    .height(Length::Fixed(560.));
    settings_modal_overlay(Some(base), close, card)
}
