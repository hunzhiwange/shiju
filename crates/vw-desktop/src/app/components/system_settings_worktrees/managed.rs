//! Project groups for gateway-registered checkouts only.
use super::{Event, message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::worktrees::ManagedMessage;
use crate::app::{Message, i18n::I18n, state::ManagedWorktreesState};
use iced::widget::{button, column, row, text, tooltip};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::worktree::{
    WorktreeProtectionReason as Reason, WorktreeStatus,
};
fn event(value: ManagedMessage) -> Message {
    message(Event::Managed(value))
}
fn protection(reason: &Reason) -> &'static str {
    match reason {
        Reason::Pinned => "已固定",
        Reason::Busy => "正在使用",
        Reason::Initializing => "正在初始化",
        Reason::Pool => "任务池工作树",
        Reason::Unknown => "归属待核实",
        Reason::SharedRepository => "共享仓库保护",
    }
}
pub(crate) fn view<'a>(state: &'a ManagedWorktreesState, i18n: I18n) -> Element<'a, Message> {
    let mut content = column![
        row![
            text(i18n.__("托管的工作树")).size(18).width(Length::Fill),
            button(text(i18n.__("刷新")))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press(event(ManagedMessage::Refresh))
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(12)
    .padding([0, 7]);
    if state.loading {
        content = content.push(text(i18n.__("正在加载…")).size(13));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    if !state.loading && state.error.is_none() && state.groups.is_empty() {
        content = content
            .push(text(i18n.__("暂无托管工作树。")).size(13).style(settings_muted_text_style));
    }
    for group in &state.groups {
        let project_id = group.project.id.as_ref().to_owned();
        let mut body = column![
            button(text(format!(
                "{} {} ({})",
                if group.collapsed { "▸" } else { "▾" },
                group.project.name,
                group.items.len()
            )))
            .width(Length::Fill)
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press(event(ManagedMessage::Toggle(project_id.clone())))
        ]
        .spacing(10);
        if !group.collapsed {
            if let Some(error) = &group.error {
                body = body.push(settings_error_banner(error.clone()));
            } else if group.items.is_empty() {
                body = body.push(
                    text(i18n.__("暂无托管工作树。")).size(13).style(settings_muted_text_style),
                );
            }
            for item in &group.items {
                let tree = &item.worktree;
                let title = item
                    .sessions
                    .iter()
                    .max_by_key(|s| s.updated_at_ms)
                    .map(|s| s.title.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(if item.title.is_empty() { &tree.name } else { &item.title });
                let status = match tree.status {
                    WorktreeStatus::Creating => "正在创建",
                    WorktreeStatus::Ready => "就绪",
                    WorktreeStatus::Busy => "正在使用",
                    WorktreeStatus::Error => "异常",
                    WorktreeStatus::Removed => "已移除",
                };
                let mut details = column![
                    text(title).size(14),
                    text(tree.directory.as_str()).size(12).style(settings_muted_text_style),
                    text(i18n.__(status)).size(12)
                ]
                .spacing(5)
                .width(Length::Fill);
                for reason in &item.protection_reasons {
                    details = details.push(
                        text(i18n.__(protection(reason))).size(12).style(settings_muted_text_style),
                    );
                }
                let trusted = tree.project_id == group.project.id
                    && tree.directory != group.project.directory
                    && group.error.is_none();
                let ready = trusted && !state.loading && tree.status == WorktreeStatus::Ready;
                let open = button(text(i18n.__("新建聊天")))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press_maybe((ready && state.opening.is_none()).then(|| {
                        event(ManagedMessage::Open {
                            project: project_id.clone(),
                            worktree: tree.id.as_ref().to_owned(),
                        })
                    }));
                let remove = button(text(i18n.__("删除")))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press_maybe(
                        (ready
                            && item.protection_reasons.is_empty()
                            && state.preflight_target.is_none())
                        .then(|| {
                            event(ManagedMessage::Preflight {
                                project: project_id.clone(),
                                worktree: tree.id.as_ref().to_owned(),
                            })
                        }),
                    );
                body = body.push(settings_divider()).push(details).push(
                    row![
                        tooltip(
                            open,
                            text(i18n.__("在此工作树中新建聊天")),
                            tooltip::Position::Top
                        ),
                        tooltip(
                            remove,
                            text(i18n.__("删除前先检查保护状态")),
                            tooltip::Position::Top
                        )
                    ]
                    .spacing(8),
                );
                if trusted && tree.status == WorktreeStatus::Error {
                    body = body.push(
                        button(text(i18n.__("重试初始化")))
                            .style(rounded_action_btn_style)
                            .padding([8, 12])
                            .on_press_maybe(state.retrying.is_none().then(|| {
                                event(ManagedMessage::Retry {
                                    project: project_id.clone(),
                                    worktree: tree.id.as_ref().to_owned(),
                                })
                            })),
                    );
                }
                if state
                    .preflight_target
                    .as_ref()
                    .is_some_and(|(p, w)| p == &project_id && w == tree.id.as_ref())
                {
                    body = body.push(
                        text(i18n.__(if state.preflight.is_some() {
                            "删除预检已完成，尚未删除工作树。"
                        } else {
                            "正在检查删除条件…"
                        }))
                        .size(12)
                        .style(settings_muted_text_style),
                    );
                    if let Some(preflight) = &state.preflight {
                        for reason in &preflight.protection_reasons {
                            body = body.push(text(i18n.__(protection(reason))).size(12));
                        }
                    }
                }
            }
        }
        content = content.push(settings_group_card(body));
    }
    content.into()
}
