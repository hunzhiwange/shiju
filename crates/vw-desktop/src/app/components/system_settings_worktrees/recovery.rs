//! Shared confirmation dialog and persistent recovery catalogue.
use super::{Event, message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::worktrees::{ManagedMessage, recovery::RecoveryMessage};
use crate::app::{Message, state::WorktreeSettingsState};
use iced::widget::{Space, button, checkbox, column, row, text};
use iced::{Alignment, Element, Length};
fn event(value: RecoveryMessage) -> Message {
    message(Event::Recovery(value))
}

pub(crate) fn view(state: &WorktreeSettingsState) -> Element<'_, Message> {
    let recovery = &state.recovery;
    let busy = recovery.pending.is_some();
    let mut body = column![
        row![
            text("清理建议与恢复快照").size(18).width(Length::Fill),
            button(text("刷新")).style(rounded_action_btn_style).padding([8, 14]).on_press_maybe(
                (!busy && !recovery.loading).then(|| event(RecoveryMessage::Refresh))
            )
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(12)
    .padding([0, 7]);
    if recovery.loading {
        body = body.push(text("正在加载清理建议与快照…"));
    }
    if let Some(error) = &recovery.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if let Some((project, item)) = &recovery.pending {
        body = body.push(text(format!("正在处理：{project} / {item}")));
    }
    for (project, tree, result) in &recovery.outcomes {
        body = body.push(
            text(match result {
                Ok(snapshot) => {
                    format!("{project} / {tree} 已删除，恢复快照：{snapshot}；聊天历史已保留。")
                }
                Err(error) => format!("{project} / {tree} 删除失败：{error}。请重新预检。"),
            })
            .size(13),
        );
    }
    if let Some((project, worktree, directory)) = &recovery.restored {
        body = body.push(text(format!("已恢复到：{directory}")));
        let available = state.managed.groups.iter().any(|g| {
            g.project.id.as_ref() == project
                && g.error.is_none()
                && g.items.iter().any(|i| i.worktree.id.as_ref() == worktree)
        });
        body = body.push(
            button(text("在恢复的工作树中新建聊天"))
                .style(primary_action_btn_style)
                .padding([8, 14])
                .on_press_maybe((available && !busy).then(|| {
                    message(Event::Managed(ManagedMessage::Open {
                        project: project.clone(),
                        worktree: worktree.clone(),
                    }))
                })),
        );
    }
    if recovery.loaded
        && recovery.projects.is_empty()
        && recovery.error.is_none()
        && !recovery.loading
    {
        body = body.push(text("暂无项目与恢复快照。"));
    }
    for project in &recovery.projects {
        let id = project.project.id.as_ref().to_owned();
        let mut group = column![text(project.project.name.clone()).size(16)].spacing(10);
        match &project.proposal {
            Err(error) => {
                group = group.push(settings_error_banner(format!("清理建议加载失败：{error}")))
            }
            Ok(proposal) => {
                group = group.push(
                    text(format!(
                        "清理候选 {} 项，受保护 {} 项，尚超出保留数量 {} 项",
                        proposal.candidates.len(),
                        proposal.protected.len(),
                        proposal.remaining_excess
                    ))
                    .size(13),
                );
                for candidate in &proposal.candidates {
                    group = group.push(
                        row![
                            text(candidate.title.clone()).width(Length::Fill),
                            button(text("选择并预检"))
                                .style(rounded_action_btn_style)
                                .padding([8, 12])
                                .on_press_maybe(
                                    (!busy && state.managed.preflight_target.is_none()).then(
                                        || message(Event::Managed(ManagedMessage::Preflight {
                                            project: id.clone(),
                                            worktree: candidate.id.clone()
                                        }))
                                    )
                                )
                        ]
                        .spacing(10),
                    );
                }
            }
        }
        match &project.snapshots {
            Err(error) => {
                group = group
                    .push(settings_error_banner(format!("快照加载失败：{error}；请刷新重试。")))
            }
            Ok(snapshots) => {
                if snapshots.is_empty() {
                    group = group
                        .push(text("暂无恢复快照。").size(13).style(settings_muted_text_style));
                }
                for snapshot in snapshots {
                    group = group
                        .push(text(format!("{} · {}", snapshot.id, snapshot.directory)).size(13));
                    if snapshot.has_ignored_files {
                        group = group.push(
                            text("原忽略文件未包含在快照中。")
                                .size(13)
                                .style(settings_muted_text_style),
                        );
                    }
                    group = group.push(
                        button(text(if snapshot.restored {
                            "已恢复"
                        } else {
                            "恢复 / 重试"
                        }))
                        .style(rounded_action_btn_style)
                        .padding([8, 12])
                        .on_press_maybe(
                            (!busy && !snapshot.restored && !recovery.loading).then(|| {
                                event(RecoveryMessage::Restore {
                                    project: id.clone(),
                                    snapshot: snapshot.id.clone(),
                                })
                            }),
                        ),
                    );
                }
            }
        }
        body = body.push(settings_group_card(group));
    }
    body.into()
}
pub(crate) fn overlay<'a>(
    state: &'a WorktreeSettingsState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some((project, worktree)) = &state.managed.preflight_target else {
        return base;
    };
    let busy = state.recovery.pending.is_some();
    let close = event(RecoveryMessage::Cancel);
    let header = settings_modal_header(
        text("确认删除工作树").size(20),
        settings_close_button(close.clone()),
    );
    let group = state.managed.groups.iter().find(|g| g.project.id.as_ref() == project);
    let item = group.and_then(|g| g.items.iter().find(|i| i.worktree.id.as_ref() == worktree));
    let mut body = column![
        text(format!("项目：{}", group.map(|g| g.project.name.as_str()).unwrap_or(project))),
        text(format!("工作树：{worktree}")),
        text("删除保留聊天历史。删除前由服务端创建恢复快照，忽略文件不在快照范围内。").size(13)
    ]
    .spacing(12);
    if let Some(item) = item {
        body = body.push(text(item.worktree.directory.clone()));
        body = body.push(text(format!("关联聊天：{}", item.sessions.len())));
        for session in &item.sessions {
            body = body.push(text(session.title.clone()).size(13));
        }
    }
    let mut allowed = false;
    if let Some(preflight) = &state.managed.preflight {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        allowed = preflight.protection_reasons.is_empty()
            && u128::from(preflight.expires_at_ms) > now
            && (!preflight.has_ignored_files || state.recovery.ignored_acknowledged);
        body = body.push(text(if preflight.has_changes {
            "快照将保留已跟踪修改和未跟踪文件。"
        } else {
            "删除前将创建恢复快照。"
        }));
        if !preflight.protection_reasons.is_empty() {
            body = body.push(settings_error_banner("工作树当前受保护，禁止删除。"));
        }
        if preflight.has_ignored_files {
            body = body.push(
                checkbox(state.recovery.ignored_acknowledged)
                    .label("我确认忽略文件将永久丢失，无法从快照恢复")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(
                        (!busy).then_some(|value| event(RecoveryMessage::Ignored(value))),
                    ),
            );
        }
    } else {
        body = body.push(text("正在重新检查删除条件…"));
    }
    if let Some(error) = &state.recovery.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text("取消"))
            .style(rounded_action_btn_style)
            .padding([8, 14])
            .on_press_maybe((!busy).then_some(close.clone())),
        button(text("重新预检")).style(rounded_action_btn_style).padding([8, 14]).on_press_maybe(
            (!busy && state.managed.preflight.is_some()).then(|| message(Event::Managed(
                ManagedMessage::Preflight { project: project.clone(), worktree: worktree.clone() }
            )))
        ),
        button(text(if busy { "正在删除…" } else { "确认删除" }))
            .style(primary_action_btn_style)
            .padding([8, 14])
            .on_press_maybe((allowed && !busy).then(|| event(RecoveryMessage::Confirm)))
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_layout(header, body, Some(footer.into())).width(720).height(580),
    )
}
