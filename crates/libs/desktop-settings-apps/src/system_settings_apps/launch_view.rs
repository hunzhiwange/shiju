use super::launch::{LaunchChoice, LaunchMessage};

use desktop_ui_controls::system_settings_common::*;
use super::launch::LaunchState;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
use vw_api_app_identity::{LaunchIdentityKind, LaunchSessionPhase};

pub fn overlay<'a, C: Clone + 'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &'a LaunchState,
    host: &'a str,
    base: Element<'a, M>,
    heading: impl Fn(&'static str, &'static str, u16) -> Element<'a, M>,
    event: impl Fn(LaunchMessage<C>) -> M + Copy + 'static,
) -> Element<'a, M> {
    let Some(request) = &state.request else {
        return base;
    };
    let close = event(LaunchMessage::Close);
    let editable = !state.busy && !state.accepted;
    let mut body = column![
        text(i18n.sprintf("应用：%s · 实例：%s", &[format!("{:?}", request.app_id).into(), format!("{}", request.installation_id.0).into()]))
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    if state.busy {
        body = body.push(text(i18n.__("正在等待确认…")).size(13));
    }
    if state.accepted {
        body = body.push(
            text(match state.phase {
                Some(LaunchSessionPhase::Completed) => i18n.__("CLI 已退出，执行端回写与清理完成"),
                Some(LaunchSessionPhase::Failed) => i18n.__("CLI 会话失败；请检查网关连接与账号状态"),
                Some(LaunchSessionPhase::Starting | LaunchSessionPhase::Running) => {
                    i18n.__("CLI 会话运行中；关闭终端将终止并回收进程")
                }
                None => i18n.__("终端已接受启动，尚未确认 CLI 完成"),
            })
            .size(13),
        );
    }
    if state.identities_loading {
        body = body.push(text(i18n.__("正在读取所选主机可用身份…")).size(13));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }

    let mut identities = column![settings_field_label(i18n.__("启动身份"))].spacing(8);
    if state.identities_ready {
        let choices = std::iter::once(LaunchChoice::ExistingConfiguration)
            .chain(state.identities.iter().cloned().map(LaunchChoice::Identity));
        identities = identities.push(
            settings_select(choices, state.selected.clone(), move |v| event(LaunchMessage::Select(v)))
                .format_labels(|choice| match choice {
                    LaunchChoice::ExistingConfiguration => i18n.__("主机现有 CLI 配置（可能使用已登录身份）").to_owned(),
                    LaunchChoice::Identity(identity) => i18n.sprintf(
                        match identity.kind {
                            LaunchIdentityKind::Provider => "供应商：%s",
                            LaunchIdentityKind::OfficialAccount => "官方账号：%s",
                        },
                        &[identity.label.clone().into()],
                    ),
                })
                .placeholder(i18n.__("选择主机现有配置、供应商或官方账号"))
                .disabled(!editable)
                .width(Length::Fill),
        );
        for (kind, label) in [
            (LaunchIdentityKind::Provider, i18n.__("暂无可用供应商")),
            (LaunchIdentityKind::OfficialAccount, i18n.__("暂无可用官方账号")),
        ] {
            if !state.identities.iter().any(|i| i.kind == kind) {
                identities = identities.push(text(label).size(12).style(settings_muted_text_style));
            }
        }
    } else if let Some(selected) = &state.selected {
        identities = identities.push(text(selected.to_string()).size(13));
    }

    let mut args = column![heading(i18n.__("附加参数"), i18n.__("每项一个参数，原样传递；不解释 shell。"), 17)]
    .spacing(8);
    for (index, value) in state.args.iter().enumerate() {
        args = args.push(
            row![
                settings_text_input(i18n.__("参数内容"), value).on_input_maybe(
                    editable.then_some(move |v| event(LaunchMessage::Args(index, v)))
                ),
                button(text(i18n.__("移除")).size(SETTINGS_CONTROL_TEXT_SIZE))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(editable.then_some(event(LaunchMessage::RemoveArg(index)))),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        );
    }
    args = args.push(
        button(text(i18n.__("添加参数")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(event(LaunchMessage::AddArg))),
    );
    let cwd = column![
        settings_field_label(i18n.__("工作目录")),
        settings_text_input(i18n.__("所选主机上的绝对目录"), &request.cwd)
            .on_input_maybe(editable.then_some(move |v| event(LaunchMessage::Cwd(v)))),
    ]
    .spacing(6);
    let body = body.push(identities).push(cwd).push(settings_divider()).push(args);

    let header = settings_modal_header(
        settings_page_heading(i18n.__("终端启动"), i18n.__("默认保留权限检查；此表单不提供跳过权限。")),
        settings_close_button(close.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("关闭")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(close.clone())),
        button(
            text(if host == local_app_auth::HOST_ID {
                i18n.__("在本机启动")
            } else {
                i18n.__("在所选主机启动")
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
        text(i18n.__(state.blocked_reason().unwrap_or("准备就绪，点击启动将在所选主机打开终端。")))
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
