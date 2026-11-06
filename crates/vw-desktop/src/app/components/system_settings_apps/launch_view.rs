use super::{LaunchMessage, event};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, state::AppsCatalogState};
use iced::{
    Element, Length,
    widget::{button, column, container, row, scrollable, text, text_input},
};
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
        status = status.push(settings_success_banner("执行主机已接受终端启动"));
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
        text("身份：使用所选执行主机的现有 CLI 配置。可能使用该主机已登录身份，请确认后启动。")
            .size(13),
        button(text("供应商账号（接口未就绪）")).padding([8, 12]).style(rounded_action_btn_style),
        button(text("官方账号（接口未就绪）")).padding([8, 12]).style(rounded_action_btn_style),
        text("尚无可用账号列表及账号终端交接契约，不能选择或回退其他账号。").size(12),
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
            row![
                button(text("关闭"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then_some(close.clone())),
                button(text("在所选主机启动"))
                    .padding([8, 16])
                    .style(primary_action_btn_style)
                    .on_press_maybe(
                        (!state.busy
                            && !state.accepted
                            && state.error.is_none()
                            && !request.cwd.trim().is_empty())
                        .then_some(event(LaunchMessage::Submit))
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
