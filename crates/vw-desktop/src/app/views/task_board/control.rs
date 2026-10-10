//! 任务看板控制面板模块。
//!
//! 本模块负责组织任务看板顶部控制区与 Worktree 状态面板，
//! 同时将共享辅助逻辑和像素办公室视图拆分到独立子模块中，
//! 以降低单文件体积并保持外部接口稳定。

mod helpers;
mod worktree_office;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
};
use crate::app::message::TaskBoardMessage;
use crate::app::task::WorktreeState;
use crate::app::{App, Message};

use self::helpers::{running_dots, worktree_state_label};
use self::worktree_office::build_worktree_pixel_office;
use super::common::{button_style_danger, button_style_primary, button_style_secondary};

pub(crate) fn should_render_worktree_controls(app: &App) -> bool {
    app.project_is_git_repo
}

fn status_chip_style(theme: &Theme, tone: Color) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if tone.a > 0.0 {
            tone.scale_alpha(0.14)
        } else {
            palette.background.weak.color
        })),
        border: Border {
            width: 1.0,
            color: if tone.a > 0.0 {
                tone.scale_alpha(0.30)
            } else {
                palette.background.strong.color.scale_alpha(0.68)
            },
            radius: 999.0.into(),
        },
        text_color: Some(if tone.a > 0.0 { tone } else { theme.palette().text.scale_alpha(0.72) }),
        ..Default::default()
    }
}

fn rounded_settings_panel_style(theme: &Theme, radius: f32) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = radius.into();
    style
}

fn top_controls_panel_style(theme: &Theme) -> iced::widget::container::Style {
    rounded_settings_panel_style(theme, 24.0)
}

fn control_outer_panel_style(theme: &Theme) -> iced::widget::container::Style {
    rounded_settings_panel_style(theme, 22.0)
}

fn faded_base_text_style(theme: &Theme, alpha: f32) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text.scale_alpha(alpha)),
    }
}

fn worktree_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.8)
}

fn worktree_summary_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.86)
}

fn worktree_refreshing_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.72)
}

fn worktree_action_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.82)
}

fn worktree_slot_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().background.base.text) }
}

fn worktree_slot_path_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.84)
}

fn worktree_slot_taint_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.76)
}

fn worktree_loading_text_style(theme: &Theme) -> iced::widget::text::Style {
    faded_base_text_style(theme, 0.78)
}

fn confirm_hint_text_style(_theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(Color::from_rgb8(153, 27, 27)) }
}

fn confirm_panel_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba(
            254.0 / 255.0,
            226.0 / 255.0,
            226.0 / 255.0,
            0.92,
        ))),
        border: Border { width: 1.0, color: Color::from_rgb8(248, 113, 113), radius: 6.0.into() },
        ..Default::default()
    }
}

fn worktree_action_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.base.color.scale_alpha(0.22))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.24),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn worktree_slot_row_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.base.color.scale_alpha(0.35))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.32),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn worktree_status_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.weak.color.scale_alpha(0.35))),
        border: Border {
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.4),
            radius: 8.0.into(),
        },
        text_color: Some(Color::from_rgba(0.95, 0.97, 1.0, 0.92)),
        ..Default::default()
    }
}

fn worktree_confirm_hint(confirm_kind: &str) -> &'static str {
    match confirm_kind {
        "cleanup" => "高风险确认中：红色按钮将执行一键清理；按 Esc 或点击空白处可取消",
        "delete" => "高风险确认中：红色按钮将删除所有 worktree；按 Esc 或点击空白处可取消",
        "merge" => "高风险确认中：红色按钮将执行一键合并；按 Esc 或点击空白处可取消",
        _ => "高风险确认中：按 Esc 或点击空白处可取消",
    }
}

fn worktree_action_status(action_kind: Option<&'static str>, now_ms: u64) -> String {
    match action_kind {
        Some("cleanup") => format!("正在清理{}", running_dots(now_ms)),
        Some("delete") => format!("正在删除所有 worktree{}", running_dots(now_ms)),
        Some("merge") => format!("正在一键合并{}", running_dots(now_ms)),
        Some("slot_recycle") => format!("正在回收 worktree{}", running_dots(now_ms)),
        _ => "正在处理 worktree 操作".to_string(),
    }
}

/// 构建任务看板控制面板，包含所有控制按钮和 Worktree 池状态显示。
pub fn build_control_buttons<'a>(app: &'a App) -> Element<'a, Message> {
    let status_chip = |label: String, tone: Color| -> Element<'a, Message> {
        container(text(label).size(11))
            .padding([5, 10])
            .style(move |theme: &Theme| status_chip_style(theme, tone))
            .into()
    };

    let title = container(
        row![
            text("任务看板")
                .size(20)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
            text("统一管理调度、工作池与批量任务动作。")
                .size(12)
                .width(Length::Fill)
                .style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill);

    let completed_count = app
        .task_board_tasks
        .iter()
        .filter(|task| {
            !task.deleted
                && !task.archived
                && task.status == crate::app::task::TaskStatus::Completed
        })
        .count();
    let now_ms = crate::app::time::now_ms();
    let worktree_manual_action_running = app.task_board_worktree_manual_action_kind.is_some();

    let scheduler_chip = status_chip(
        if app.task_board_executor_running {
            "调度运行中".to_string()
        } else {
            "调度待启动".to_string()
        },
        if app.task_board_executor_running {
            Color::from_rgb8(0x2E, 0xB8, 0x72)
        } else {
            Color::from_rgba8(0, 0, 0, 0.0)
        },
    );
    let completed_chip = status_chip(
        format!("待归档 {}", completed_count),
        if completed_count > 0 {
            Color::from_rgb8(0xF2, 0xA9, 0x00)
        } else {
            Color::from_rgba8(0, 0, 0, 0.0)
        },
    );
    let worktree_chip = if should_render_worktree_controls(app) && worktree_manual_action_running {
        Some(status_chip(
            format!("工作池处理中{}", running_dots(now_ms)),
            Color::from_rgb8(0xFF, 0x7A, 0x00),
        ))
    } else {
        None
    };

    let settings_btn = button(text("看板设置").size(12))
        .on_press(Message::TaskBoard(TaskBoardMessage::OpenSettingsModal))
        .padding([6, 12])
        .style(button_style_secondary);

    let (exec_btn_label, exec_msg) = if app.task_board_executor_running {
        ("停止任务调度".to_string(), TaskBoardMessage::StopExecution)
    } else {
        ("开始任务调度".to_string(), TaskBoardMessage::StartExecution)
    };
    let exec_btn = button(text(exec_btn_label).size(12))
        .on_press(Message::TaskBoard(exec_msg))
        .padding([6, 12])
        .style(if app.task_board_executor_running {
            button_style_danger
        } else {
            button_style_primary
        });

    let new_task_btn = button(text("+ 新建任务").size(12))
        .on_press(Message::TaskBoard(TaskBoardMessage::CreateTask))
        .padding([6, 12])
        .style(button_style_primary);

    let archive_completed_btn = button(text(format!("归档已完成 {}", completed_count)).size(12))
        .on_press_maybe(
            (completed_count > 0)
                .then_some(Message::TaskBoard(TaskBoardMessage::ArchiveCompletedTasks)),
        )
        .padding([6, 12])
        .style(button_style_secondary);

    let refresh_btn = button(text("刷新任务").size(12))
        .on_press(Message::TaskBoard(TaskBoardMessage::LoadTasks))
        .padding([6, 12])
        .style(button_style_secondary);

    let close_btn = button(text("关闭看板").size(12))
        .on_press(Message::TaskBoard(TaskBoardMessage::CloseBoard))
        .padding([6, 12])
        .style(button_style_secondary);

    let mut summary_row =
        row![scheduler_chip, completed_chip].spacing(8).align_y(Alignment::Center);
    if let Some(worktree_chip) = worktree_chip {
        summary_row = summary_row.push(worktree_chip);
    }

    let controls_row = row![
        exec_btn,
        new_task_btn,
        settings_btn,
        archive_completed_btn,
        refresh_btn,
        close_btn
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let controls_scroll = scrollable(controls_row)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Shrink)
        .width(Length::Fill);

    let top_controls = container(
        column![row![title, summary_row].spacing(12).align_y(Alignment::Center), controls_scroll]
            .spacing(12),
    )
    .padding(Padding::from([16, 18]))
    .style(top_controls_panel_style);

    let mut content = column![top_controls].spacing(12);
    if should_render_worktree_controls(app) {
        if let Some(snapshot) = &app.task_board_worktree_snapshot {
            let title_line = text("工作池").size(13).style(worktree_title_text_style);
            let lock_text = if snapshot.merge_target_locks.is_empty() {
                "合并锁: 无".to_string()
            } else {
                format!(
                    "合并锁: {}",
                    snapshot
                        .merge_target_locks
                        .iter()
                        .map(|(target, task_id)| format!("{}<-{}", target, task_id))
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            };
            let summary_line = text(format!(
                "待复用:{} 忙碌:{} 污染:{} 回收中:{} 失效:{} | 基线分支:{} | 目录:{} | {}",
                snapshot.idle_count,
                snapshot.busy_count,
                snapshot.tainted_count,
                snapshot.recycling_count,
                snapshot.dead_count,
                snapshot.base_branch,
                snapshot.pool_root,
                lock_text
            ))
            .size(12)
            .style(worktree_summary_text_style);

            let toggle_label = if app.task_board_worktree_panel_expanded {
                "收起槽位明细"
            } else {
                "展开槽位明细"
            };
            let toggle_btn = button(text(toggle_label).size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::ToggleWorktreePanelExpanded))
                .padding([4, 8])
                .style(button_style_secondary);
            let office_toggle_label = if app.task_board_worktree_pixel_office {
                "切换列表视图"
            } else {
                "切换像素办公室"
            };
            let office_toggle_btn = button(text(office_toggle_label).size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::ToggleWorktreePixelOffice(
                    !app.task_board_worktree_pixel_office,
                )))
                .padding([4, 8])
                .style(button_style_secondary);
            let clean_btn_label = if app.task_board_worktree_manual_action_kind == Some("cleanup") {
                format!("一键清理进行中{}", running_dots(now_ms))
            } else if app.task_board_worktree_manual_confirm_kind == Some("cleanup") {
                "确认一键清理".to_string()
            } else {
                "一键清理".to_string()
            };
            let clean_btn = button(text(clean_btn_label).size(12))
                .on_press_maybe(
                    (!worktree_manual_action_running)
                        .then_some(Message::TaskBoard(TaskBoardMessage::CleanAllWorktreesPressed)),
                )
                .padding([4, 8])
                .style(if app.task_board_worktree_manual_confirm_kind == Some("cleanup") {
                    button_style_danger
                } else {
                    button_style_secondary
                });
            let delete_btn_label = if app.task_board_worktree_manual_action_kind == Some("delete") {
                format!("删除所有 worktree 中{}", running_dots(now_ms))
            } else if app.task_board_worktree_manual_confirm_kind == Some("delete") {
                "确认删除所有 worktree".to_string()
            } else {
                "一键删除".to_string()
            };
            let delete_btn = button(text(delete_btn_label).size(12))
                .on_press_maybe(
                    (!worktree_manual_action_running)
                        .then_some(Message::TaskBoard(TaskBoardMessage::DeleteAllWorktreesPressed)),
                )
                .padding([4, 8])
                .style(if app.task_board_worktree_manual_confirm_kind == Some("delete") {
                    button_style_danger
                } else {
                    button_style_secondary
                });
            let merge_btn_label = if app.task_board_worktree_manual_action_kind == Some("merge") {
                format!("一键合并进行中{}", running_dots(now_ms))
            } else if app.task_board_worktree_manual_confirm_kind == Some("merge") {
                "确认一键合并".to_string()
            } else {
                "一键合并".to_string()
            };
            let merge_btn = button(text(merge_btn_label).size(12))
                .on_press_maybe((!worktree_manual_action_running).then_some(Message::TaskBoard(
                    TaskBoardMessage::CommitMergeAllWorktreesPressed,
                )))
                .padding([4, 8])
                .style(if app.task_board_worktree_manual_confirm_kind == Some("merge") {
                    button_style_danger
                } else {
                    button_style_secondary
                });
            let mut title_row = row![title_line, clean_btn, delete_btn, merge_btn]
                .spacing(12)
                .align_y(Alignment::Center);
            if app.task_board_worktree_manual_confirm_kind.is_some() {
                let cancel_btn = button(text("取消确认").size(12))
                    .on_press(Message::TaskBoard(TaskBoardMessage::CancelWorktreeManualConfirm))
                    .padding([4, 8])
                    .style(button_style_secondary);
                title_row = title_row.push(cancel_btn);
            }
            title_row = title_row.push(office_toggle_btn).push(toggle_btn);
            if app.task_board_worktree_snapshot_loading {
                let loading_text = format!("后台刷新中{}", running_dots(now_ms));
                title_row = title_row
                    .push(text(loading_text).size(12).style(worktree_refreshing_text_style));
            }

            let mut panel_col = column![title_row, summary_line].spacing(4);
            if let Some(confirm_kind) = app.task_board_worktree_manual_confirm_kind {
                let confirm_panel = container(
                    text(worktree_confirm_hint(confirm_kind))
                        .size(11)
                        .style(confirm_hint_text_style),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(confirm_panel_style);
                panel_col = panel_col.push(confirm_panel);
            }
            if worktree_manual_action_running || !app.task_board_worktree_action_logs.is_empty() {
                let action_status =
                    worktree_action_status(app.task_board_worktree_manual_action_kind, now_ms);
                let status_panel =
                    container(text(action_status).size(11).style(worktree_action_text_style))
                        .padding([8, 10])
                        .width(Length::Fill)
                        .style(worktree_action_panel_style);
                panel_col = panel_col.push(status_panel);
            }
            if app.task_board_worktree_panel_expanded {
                let mut slots_col = column![].spacing(6);
                for slot in &snapshot.slots {
                    let owner = slot.leased_task_id.as_deref().unwrap_or("-");
                    let taint = slot.taint_reason.as_deref().unwrap_or("-");
                    let recycle_btn = button(text("回收").size(11))
                        .on_press(Message::TaskBoard(
                            TaskBoardMessage::RecycleWorktreeSlotPressed {
                                slot_id: slot.id.clone(),
                            },
                        ))
                        .padding([3, 8])
                        .style(
                            if matches!(slot.state, WorktreeState::Tainted | WorktreeState::Dead) {
                                button_style_danger
                            } else {
                                button_style_secondary
                            },
                        );
                    let slot_row = container(
                        column![
                            row![
                                text(format!(
                                    "{} [{}] 分支={} 任务={} ",
                                    slot.id,
                                    worktree_state_label(slot.state),
                                    slot.branch,
                                    owner,
                                ))
                                .size(12)
                                .width(Length::Fill)
                                .style(worktree_slot_title_text_style),
                                recycle_btn,
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            text(format!("路径={}", slot.path))
                                .size(12)
                                .style(worktree_slot_path_text_style),
                            text(format!("污染原因={}", taint))
                                .size(12)
                                .style(worktree_slot_taint_text_style),
                        ]
                        .spacing(2),
                    )
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(worktree_slot_row_style);
                    slots_col = slots_col.push(slot_row);
                }
                panel_col = panel_col.push(slots_col);
            }

            let status_panel = container(panel_col)
                .padding([8, 10])
                .width(Length::Fill)
                .style(worktree_status_panel_style);
            content = content.push(status_panel);
            if app.task_board_worktree_pixel_office {
                content = content.push(build_worktree_pixel_office(app, snapshot));
            }
        } else if app.project_path.is_some() && app.task_board_worktree_snapshot_loading {
            let loading_panel = container(
                text("Worktree 池后台加载中...").size(12).style(worktree_loading_text_style),
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(worktree_status_panel_style);
            content = content.push(loading_panel);
        }
    }

    container(content).padding([18, 20]).width(Length::Fill).style(control_outer_panel_style).into()
}
