use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::aggregation_list::clients::{Action, Event, Session, message},
};
use iced::{
    Element,
    widget::{button, column, container, row, text},
};
pub(crate) fn overlay<'a>(s: &'a Session, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = message(Event::Close);
    let header = settings_modal_header(
        text("Codex 客户端状态").size(20),
        settings_close_button(close.clone()),
    );
    let mut body = column![].spacing(12);
    if s.pending {
        body = body.push(text("正在请求执行主机…"));
    }
    if let Some(error) = s.error {
        body = body.push(settings_error_banner(error));
    }
    if s.confirming {
        body = body.push(settings_panel(column![text("确认重启托管守护进程？").size(18), text("重启会中断守护进程中正在运行的任务。桌面版与编辑器不会自动重启。确认后仍会核对权限、修订和托管安装身份。")].spacing(10)));
    }
    if let Some(state) = &s.snapshot {
        let title = if state.unverified {
            "无法完整核实客户端，可能仍缓存旧状态。"
        } else if state.auth {
            "客户端可能缓存旧登录；这是推断。"
        } else if state.daemon || state.others {
            "客户端仍缓存旧目录。"
        } else {
            "当前探测未发现陈旧客户端；不代表所有客户端已刷新。"
        };
        body = body.push(text(title));
        if state.daemon {
            body = body.push(text("命令行托管守护进程：可在独立授权与再次核验身份后重启。"));
        }
        if state.others {
            body = body
                .push(text("桌面版：请彻底退出后重新打开。编辑器插件：请完全退出编辑器再重开。"));
        }
        if state.unverified {
            body = body.push(text(
                "未知探测不代表已刷新；确认提醒仅记录此修订，不会重启或清除已知陈旧进程。",
            ));
        }
    }
    if s.acknowledged {
        body = body.push(text("已确认此修订提醒；未执行重启，客户端刷新状态仍以探测为准。"));
    }
    if let Some(label) = s.result_label() {
        body = body.push(text(label));
    }
    let mut actions = row![].spacing(10);
    if s.confirming && s.operation.is_none() {
        actions = actions
            .push(
                button(text("确认重启"))
                    .padding([8, 14])
                    .style(primary_action_btn_style)
                    .on_press_maybe(s.can_restart().then(|| message(Event::Act(Action::Restart)))),
            )
            .push(
                button(text("取消"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!s.pending).then(|| message(Event::Back))),
            );
    } else if s.can_restart() {
        actions = actions.push(
            button(text("重启守护进程"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press(message(Event::AskRestart)),
        );
    }
    if s.running() || s.operation.is_some() {
        actions = actions.push(
            button(text("查询进度"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.pending).then(|| message(Event::Act(Action::Poll)))),
        );
        if s.running() {
            actions = actions.push(
                button(text("请求取消"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        (!s.pending
                            && s.operation.as_ref().is_some_and(|p| !p.cancellation_requested))
                        .then(|| message(Event::Act(Action::Cancel))),
                    ),
            );
        }
    } else if !s.confirming
        && s.error.is_none()
        && !s.acknowledged
        && s.snapshot.as_ref().is_some_and(|state| state.unverified && !state.revision.is_empty())
    {
        actions = actions.push(
            button(text("确认提醒"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.pending).then(|| message(Event::Act(Action::Ack)))),
        );
    }
    let footer = column![
        actions,
        row![
            button(text("重新探测"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (!s.pending && !s.running()).then(|| message(Event::Act(Action::Read)))
                ),
            button(text("返回目录"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!s.pending && !s.running()).then_some(close.clone()))
        ]
        .spacing(10)
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(700).height(560);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "aggregation_clients_render_tests.rs"]
mod render_tests;
