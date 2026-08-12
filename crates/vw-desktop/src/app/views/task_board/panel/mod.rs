//! 任务看板侧栏面板模块，负责任务编辑、执行器选择和日志弹窗界面。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use std::fmt;

use iced::widget::{
    Space, button, checkbox, column, container, pick_list, row, scrollable, text, text_editor,
    text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_close_button, settings_divider, settings_muted_text_style,
    settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
};
use crate::app::message::TaskBoardMessage;
use crate::app::state::{MAIN_AGENT_KEY, TaskBoardDetailTab};
use crate::app::task::{Task, TaskSplitMode};
use crate::app::{App, Message};
use vw_config_types::agent::is_retired_builtin_agent_key;

use super::common::{button_style_primary, button_style_secondary, button_style_success};

mod executor_selector;
mod logs_modal;
mod model_selector;
mod reasoning_selector;
mod styles;
mod subtask_editor;

/// 重新导出 executor_selector::{build_bulk_executor_selector, build_executor_selector}，作为上层模块访问该视图能力的稳定入口。
pub(crate) use executor_selector::{build_bulk_executor_selector, build_executor_selector};
/// 重新导出 model_selector::build_bulk_model_selector，作为上层模块访问该视图能力的稳定入口。
pub(crate) use model_selector::build_bulk_model_selector;
/// 重新导出 model_selector::build_model_selector，作为上层模块访问该视图能力的稳定入口。
pub(crate) use model_selector::build_model_selector;
pub(crate) use reasoning_selector::{
    build_bulk_reasoning_effort_selector, build_reasoning_effort_selector,
};
use styles::{editor_style, input_label, input_style, panel_container_style};
use subtask_editor::{build_draft_mode_subtasks, build_edit_mode_subtasks, build_task_logs};

const TASK_PANEL_SCROLLBAR_WIDTH: f32 = 4.0;
const TASK_PANEL_SCROLLBAR_GUTTER: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskBoardAgentOption {
    key: String,
    label: String,
}

impl fmt::Display for TaskBoardAgentOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn task_status_tag_colors(status: crate::app::task::TaskStatus) -> (Color, Color) {
    match status {
        crate::app::task::TaskStatus::Pool => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
        crate::app::task::TaskStatus::Pending => {
            (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254))
        }
        crate::app::task::TaskStatus::Planning => {
            (Color::from_rgb8(79, 70, 229), Color::from_rgb8(224, 231, 255))
        }
        crate::app::task::TaskStatus::Running => {
            (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255))
        }
        crate::app::task::TaskStatus::Failed => {
            (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226))
        }
        crate::app::task::TaskStatus::Paused => {
            (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195))
        }
        crate::app::task::TaskStatus::CodeComplete => {
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229))
        }
        crate::app::task::TaskStatus::CodeReview => {
            (Color::from_rgb8(217, 119, 6), Color::from_rgb8(254, 243, 199))
        }
        crate::app::task::TaskStatus::PrSubmitted => {
            (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254))
        }
        crate::app::task::TaskStatus::Completed => {
            (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231))
        }
        crate::app::task::TaskStatus::Archived => {
            (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249))
        }
        // 自定义状态暂用中性灰，待工作流迁移任务接管自定义列样式。
        crate::app::task::TaskStatus::Custom(_) => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
    }
}

fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

fn format_duration_ms(duration_ms: u64) -> String {
    let secs = (duration_ms / 1000) as i64;
    let value = vw_shared::util::format_duration(secs);
    if value.is_empty() { "0s".to_string() } else { value }
}

fn running_dots(now_ms: u64) -> &'static str {
    match ((now_ms / 1000) % 3) as u8 {
        0 => "·",
        1 => "··",
        _ => "···",
    }
}

fn segmented_button_style(
    theme: &Theme,
    active: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let active_bg = if is_dark {
        theme.palette().primary.scale_alpha(0.18)
    } else {
        theme.palette().primary.scale_alpha(0.10)
    };
    let background = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(if active {
            active_bg
        } else if is_dark {
            p.background.weak.color.scale_alpha(0.84)
        } else {
            Color::WHITE.scale_alpha(0.92)
        })),
        iced::widget::button::Status::Pressed => {
            Some(Background::Color(p.background.strong.color.scale_alpha(if is_dark {
                0.82
            } else {
                0.26
            })))
        }
        _ => Some(Background::Color(if active {
            active_bg
        } else if is_dark {
            p.background.base.color.scale_alpha(0.54)
        } else {
            Color::WHITE.scale_alpha(0.76)
        })),
    };

    iced::widget::button::Style {
        background,
        text_color: if active { theme.palette().primary } else { theme.palette().text },
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if active {
                theme.palette().primary.scale_alpha(if is_dark { 0.68 } else { 0.34 })
            } else {
                p.background.strong.color.scale_alpha(0.62)
            },
        },
        shadow: if active {
            iced::Shadow {
                color: theme.palette().primary.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 8.0),
                blur_radius: 18.0,
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

fn section_block<'a>(
    title: &'a str,
    description: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        text(title)
            .size(13)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        text(description).size(12).style(settings_muted_text_style),
        settings_panel(content.into()),
    ]
    .spacing(8)
    .into()
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `is_edit_mode`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_task_panel<'a>(app: &'a App, is_edit_mode: bool) -> Element<'a, Message> {
    let current_task = if is_edit_mode { app.task_board_viewing_logs.as_ref() } else { None };
    let (panel_title, panel_description, task_id_text, close_msg) = if is_edit_mode {
        let title = "任务详情";
        let id = current_task.map(|t| t.id.as_str()).unwrap_or("");
        (
            title,
            "查看运行状态、调整执行参数，并保持日志与子任务在同一侧面板内。",
            Some(id),
            TaskBoardMessage::CloseTaskLogs,
        )
    } else {
        (
            "新建任务",
            "集中填写提示词、模型、执行器与导入内容。",
            None,
            TaskBoardMessage::CreateTaskCancelled,
        )
    };

    let close_btn = build_close_button(close_msg);
    let header_row = build_header_row(panel_title, task_id_text, current_task, close_btn);

    let mut main_col = column![
        header_row,
        Space::new().height(6.0),
        text(panel_description).size(12).style(settings_muted_text_style),
        Space::new().height(18.0)
    ]
    .spacing(0)
    .width(Length::Fill);

    if is_edit_mode {
        main_col = main_col.push(build_detail_tabs(app)).push(Space::new().height(14.0));

        if app.task_board_detail_tab == TaskBoardDetailTab::Logs {
            main_col = main_col.push(build_task_logs(app));
        } else {
            if let Some(reason_row) = current_task.and_then(build_failed_reason_row) {
                main_col = main_col.push(reason_row).push(Space::new().height(12.0));
            }
            if let Some(task) = current_task
                && let Some(worktree_row) = build_selected_worktree_row(task)
            {
                main_col = main_col.push(worktree_row).push(Space::new().height(12.0));
            }
            if let Some(task) = current_task
                && let Some(merge_lock_row) = build_merge_lock_row(app, task)
            {
                main_col = main_col.push(merge_lock_row).push(Space::new().height(12.0));
            }

            let priority_field = build_priority_field(app, true);
            let model_field = build_model_selector(app, true);
            let agent_field = build_agent_field(app, true);
            let executor_field = build_executor_selector(app, true);
            let reasoning_effort_field = build_reasoning_effort_selector(app, true);
            let task_split_field = build_task_split_mode_selector(app, true);
            let prompt_field = build_prompt_field(app);

            main_col = main_col.push(section_block(
                "任务配置",
                "编辑提示词、模型、代理、执行器、任务拆分与优先级。",
                column![
                    prompt_field,
                    settings_divider(),
                    row![
                        container(agent_field).width(Length::FillPortion(1)),
                        container(executor_field).width(Length::FillPortion(1)),
                        container(reasoning_effort_field).width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                    row![
                        container(model_field).width(Length::FillPortion(1)),
                        container(task_split_field).width(Length::FillPortion(1)),
                        container(priority_field).width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                ]
                .spacing(14),
            ));

            if let Some(task) = current_task {
                main_col = build_edit_mode_content(app, task, main_col);
            }
        }
    } else {
        main_col = main_col.push(build_mode_toggle(app)).push(Space::new().height(12.0));
        if app.task_board_is_import_mode {
            main_col = main_col.push(section_block(
                "批量导入",
                "支持 JSON、CSV、TSV，适合一次性导入多条任务。",
                build_import_field(app),
            ));
        } else {
            let priority_field = build_priority_field(app, false);
            let model_field = build_model_selector(app, false);
            let agent_field = build_agent_field(app, false);
            let executor_field = build_executor_selector(app, false);
            let reasoning_effort_field = build_reasoning_effort_selector(app, false);
            let task_split_field = build_task_split_mode_selector(app, false);
            let prompt_field = build_prompt_field(app);
            main_col = main_col.push(section_block(
                "任务配置",
                "填写提示词并选择模型、代理、执行器、任务拆分与优先级。",
                column![
                    prompt_field,
                    settings_divider(),
                    row![
                        container(agent_field).width(Length::FillPortion(1)),
                        container(executor_field).width(Length::FillPortion(1)),
                        container(reasoning_effort_field).width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                    row![
                        container(model_field).width(Length::FillPortion(1)),
                        container(task_split_field).width(Length::FillPortion(1)),
                        container(priority_field).width(Length::FillPortion(1)),
                    ]
                    .spacing(12),
                ]
                .spacing(14),
            ));
        }
        main_col = build_draft_mode_content(app, main_col);
    }

    let panel_padding =
        iced::Padding { top: 0.0, right: TASK_PANEL_SCROLLBAR_GUTTER, bottom: 0.0, left: 0.0 };
    let content: Element<'_, Message> =
        if is_edit_mode && app.task_board_detail_tab == TaskBoardDetailTab::Logs {
            container(main_col.height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(panel_padding)
                .into()
        } else {
            scrollable(container(main_col).width(Length::Fill).padding(panel_padding))
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(TASK_PANEL_SCROLLBAR_WIDTH)
                        .scroller_width(TASK_PANEL_SCROLLBAR_WIDTH),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };
    let panel_width = if is_edit_mode { Length::Fixed(560.0) } else { Length::FillPortion(1) };

    container(content)
        .padding(20)
        .style(panel_container_style)
        .width(panel_width)
        .height(Length::Fill)
        .into()
}

fn build_close_button(close_msg: TaskBoardMessage) -> iced::Element<'static, Message> {
    settings_close_button(Message::TaskBoard(close_msg))
}

fn build_detail_tabs(app: &App) -> Element<'_, Message> {
    row![
        build_detail_tab_button("基本", TaskBoardDetailTab::Basic, app.task_board_detail_tab),
        build_detail_tab_button("日志", TaskBoardDetailTab::Logs, app.task_board_detail_tab),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn build_detail_tab_button(
    label: &str,
    tab: TaskBoardDetailTab,
    active_tab: TaskBoardDetailTab,
) -> iced::widget::Button<'_, Message> {
    button(text(label).size(13))
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectDetailTab(tab)))
        .padding([6, 14])
        .style(move |theme: &Theme, status| {
            segmented_button_style(theme, active_tab == tab, status)
        })
}

fn build_header_row<'a>(
    panel_title: &'a str,
    task_id_text: Option<&'a str>,
    task: Option<&'a Task>,
    close_btn: iced::Element<'a, Message>,
) -> iced::widget::Row<'a, Message> {
    let mut title_row = row![].spacing(8).align_y(Alignment::Center).width(Length::Shrink);
    let now = now_ms();

    if let Some(id) = task_id_text {
        title_row = title_row.push(text(id).size(11).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().background.base.text) }
        }));
    }

    title_row = title_row.push(
        text(panel_title)
            .size(16)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
    );

    if let Some(status) = task.map(|value| value.status) {
        let (status_color, status_bg_color) = task_status_tag_colors(status);
        let status_text = if status == crate::app::task::TaskStatus::Running {
            format!("{} {}", status.label(), running_dots(now))
        } else {
            status.label().to_string()
        };
        let status_tag = container(text(status_text).size(10))
            .height(Length::Fixed(20.0))
            .align_y(iced::alignment::Vertical::Center)
            .padding([0, 8])
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(status_bg_color)),
                border: Border { radius: 999.0.into(), ..Default::default() },
                text_color: Some(status_color),
                ..Default::default()
            });
        title_row = title_row.push(status_tag);
    }

    if let Some(duration_ms) = task.and_then(|value| value.display_execution_duration_ms(now)) {
        let label =
            if task.is_some_and(|value| value.status == crate::app::task::TaskStatus::Running) {
                format!("执行中 {}", format_duration_ms(duration_ms))
            } else {
                format!("已执行 {}", format_duration_ms(duration_ms))
            };
        title_row =
            title_row.push(text(label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.85)),
            }));
    }

    row![title_row, Space::new().width(Length::Fill), close_btn,]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
}

fn build_failed_reason_row<'a>(task: &'a crate::app::task::Task) -> Option<Element<'a, Message>> {
    if task.status == crate::app::task::TaskStatus::Failed {
        let reason = task.last_error.as_deref().unwrap_or("未知错误").trim();
        if reason.is_empty() {
            return None;
        }
        let reason_text = text(format!("执行失败: {}", reason))
            .size(12)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().danger.base.color),
            });
        let retry_button = button(text("手工重试").size(12))
            .padding([6, 10])
            .style(button_style_secondary)
            .on_press(Message::TaskBoard(TaskBoardMessage::ExecuteTask {
                task_id: task.id.clone(),
            }));
        return Some(
            container(column![reason_text, retry_button].spacing(10).align_x(Alignment::Start))
                .padding([8, 10])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.danger.weak.color.scale_alpha(0.45))),
                        border: Border {
                            color: p.danger.base.color.scale_alpha(0.6),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        );
    }
    if task.status == crate::app::task::TaskStatus::Paused {
        let reason = task.pause_reason.as_deref().unwrap_or("人工暂停").trim();
        if reason.is_empty() {
            return None;
        }
        let reason_text = text(format!("暂停原因: {}", reason))
            .size(12)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().warning.base.color),
            });
        return Some(
            container(reason_text)
                .padding([8, 10])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.warning.weak.color.scale_alpha(0.45))),
                        border: Border {
                            color: p.warning.base.color.scale_alpha(0.6),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        );
    }
    None
}

fn build_selected_worktree_row<'a>(
    task: &'a crate::app::task::Task,
) -> Option<Element<'a, Message>> {
    let path = task.selected_worktree_path.as_deref()?.trim();
    if path.is_empty() {
        return None;
    }
    Some(
        container(
            column![
                text("当前工作区").size(12).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text.scale_alpha(0.78)),
                }),
                text(path).size(12).wrapping(iced::widget::text::Wrapping::Word),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.primary.weak.color.scale_alpha(0.22))),
                border: Border {
                    color: p.primary.base.color.scale_alpha(0.32),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into(),
    )
}

fn build_merge_lock_row<'a>(
    app: &'a App,
    task: &'a crate::app::task::Task,
) -> Option<Element<'a, Message>> {
    let _project_path = app.project_path.as_deref()?;
    let target_branch = task.merge_target_branch.as_deref()?.trim();
    if target_branch.is_empty() {
        return None;
    }
    let lock_holder = app.task_board_worktree_snapshot.as_ref().and_then(|snapshot| {
        snapshot
            .merge_target_locks
            .iter()
            .find(|(target, _)| target == target_branch)
            .map(|(_, task_id)| task_id.clone())
    });
    let holder_text = match lock_holder.as_deref() {
        Some(task_id) if task_id == task.id => "当前任务持有".to_string(),
        Some(task_id) => format!("占用者: {}", task_id),
        None => "当前无占用者".to_string(),
    };
    Some(
        container(
            column![
                text("Merge 锁状态").size(12).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text.scale_alpha(0.78)),
                }),
                text(format!("目标分支: {}", target_branch)).size(12),
                text(holder_text).size(12),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.secondary.weak.color.scale_alpha(0.26))),
                border: Border {
                    color: p.secondary.base.color.scale_alpha(0.32),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into(),
    )
}

fn build_priority_field<'a>(app: &'a App, is_edit_mode: bool) -> iced::Element<'a, Message> {
    let priority_msg = if is_edit_mode {
        TaskBoardMessage::UpdateEditingTaskPriority
    } else {
        TaskBoardMessage::UpdateDraftPriority
    };
    text_input("1-99999，默认999", &app.task_board_draft.priority)
        .on_input(move |v| Message::TaskBoard(priority_msg(v)))
        .padding([8, 10])
        .size(14)
        .width(Length::Fill)
        .style(input_style)
        .into()
}

fn build_task_split_mode_selector(app: &App, is_edit_mode: bool) -> iced::Element<'_, Message> {
    let selected = app.task_board_draft.task_split_mode;
    let options = TaskSplitMode::all().to_vec();
    pick_list(options, Some(selected), move |mode| {
        Message::TaskBoard(if is_edit_mode {
            TaskBoardMessage::UpdateEditingTaskSplitMode(mode)
        } else {
            TaskBoardMessage::UpdateDraftTaskSplitMode(mode)
        })
    })
    .placeholder("任务拆分")
    .padding([6, 10])
    .text_size(14)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into()
}

pub(crate) fn build_bulk_task_split_selector(app: &App) -> iced::Element<'_, Message> {
    let selected = app.task_board_bulk_task_split_mode;
    let options = TaskSplitMode::all().to_vec();
    pick_list(options, Some(selected), |mode| {
        Message::TaskBoard(TaskBoardMessage::UpdateBulkTaskSplitMode(mode))
    })
    .placeholder("任务拆分")
    .padding([6, 10])
    .text_size(14)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into()
}

fn build_agent_field(app: &App, is_edit_mode: bool) -> iced::Element<'_, Message> {
    let selected_key =
        app.task_board_draft.agent.clone().unwrap_or_else(|| MAIN_AGENT_KEY.to_string());
    let options = build_agent_options(app, &selected_key);

    let selected = options
        .iter()
        .find(|option| option.key == selected_key)
        .cloned()
        .or_else(|| options.iter().find(|option| option.key == MAIN_AGENT_KEY).cloned());

    pick_list(options, selected, move |option| {
        Message::TaskBoard(if is_edit_mode {
            TaskBoardMessage::UpdateEditingTaskAgent(Some(option.key))
        } else {
            TaskBoardMessage::UpdateDraftAgent(Some(option.key))
        })
    })
    .placeholder("选择代理")
    .padding([6, 10])
    .text_size(14)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into()
}

pub(crate) fn build_bulk_agent_selector(app: &App) -> iced::Element<'_, Message> {
    let selected_key = if app.task_board_bulk_agent.trim().is_empty() {
        MAIN_AGENT_KEY.to_string()
    } else {
        app.task_board_bulk_agent.trim().to_string()
    };
    let options = build_agent_options(app, &selected_key);
    let selected = options
        .iter()
        .find(|option| option.key == selected_key)
        .cloned()
        .or_else(|| options.iter().find(|option| option.key == MAIN_AGENT_KEY).cloned());

    pick_list(options, selected, |option| {
        Message::TaskBoard(TaskBoardMessage::BulkAgentSelected(option.key))
    })
    .placeholder("选择代理")
    .padding([6, 10])
    .text_size(14)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into()
}

fn build_agent_options(app: &App, selected_key: &str) -> Vec<TaskBoardAgentOption> {
    let mut options = Vec::new();

    for entry in &app.agents_settings.entries {
        if is_retired_builtin_agent_key(&entry.key) {
            continue;
        }
        if entry.key != MAIN_AGENT_KEY && !entry.enabled && selected_key != entry.key {
            continue;
        }

        options.push(TaskBoardAgentOption {
            key: entry.key.clone(),
            label: if entry.key == MAIN_AGENT_KEY {
                "Main".to_string()
            } else {
                entry.label.clone()
            },
        });
    }

    if !options.iter().any(|option| option.key == MAIN_AGENT_KEY) {
        options.insert(
            0,
            TaskBoardAgentOption { key: MAIN_AGENT_KEY.to_string(), label: "Main".to_string() },
        );
    }

    options
}

fn build_prompt_field<'a>(app: &'a App) -> iced::Element<'a, Message> {
    let prompt_editor = text_editor(&app.task_board_prompt_editor)
        .placeholder("输入大模型提示词...")
        .on_action(|a| Message::TaskBoard(TaskBoardMessage::PromptEditorAction(a)))
        .size(14)
        .padding([8, 10])
        .height(Length::Fixed(150.0))
        .style(editor_style);

    column![input_label("大模型提示词"), prompt_editor].spacing(6).width(Length::Fill).into()
}

fn build_mode_toggle(app: &App) -> iced::Element<'_, Message> {
    row![
        button(text("单个任务").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(false)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                segmented_button_style(theme, !app.task_board_is_import_mode, status)
            }),
        button(text("批量导入").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(true)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                segmented_button_style(theme, app.task_board_is_import_mode, status)
            }),
    ]
    .spacing(8)
    .into()
}

fn build_import_field<'a>(app: &'a App) -> iced::Element<'a, Message> {
    fn tab_button<'a>(
        title: &'a str,
        selected: bool,
        format: crate::app::task::TaskImportPromptFormat,
    ) -> iced::widget::Button<'a, Message> {
        button(text(title).size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetImportPromptFormat(format)))
            .padding([5, 10])
            .style(move |theme: &Theme, status| segmented_button_style(theme, selected, status))
    }

    let selected_priority = app
        .task_board_draft
        .priority
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(app.task_board_settings.default_priority);
    let selected_model = app.task_board_draft.model.as_str();
    let prompt_template = crate::app::message::task_board::import_prompt_template(
        app.task_board_import_prompt_format,
        selected_priority,
        selected_model,
        app.task_board_draft.acp_agent.as_deref(),
    );

    let import_tools = row![
        button(text("上传文件").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ImportFilePick))
            .padding([6, 12])
            .style(button_style_secondary),
        pick_list(vec!["JSON 示例", "CSV 示例", "TSV 示例"], None::<&str>, |selected| {
            Message::TaskBoard(TaskBoardMessage::InsertDemoData(selected.to_string()))
        })
        .placeholder("插入示例到表单")
        .padding([6, 10])
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
        button(text("清空").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ClearImportEditor))
            .padding([6, 12])
            .style(button_style_secondary)
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let import_editor = text_editor(&app.task_board_import_editor)
        .placeholder("粘贴 JSON 数组或 CSV 内容...")
        .on_action(|a| Message::TaskBoard(TaskBoardMessage::ImportEditorAction(a)))
        .size(14)
        .padding([8, 10])
        .height(Length::Fixed(300.0))
        .style(editor_style);

    let help_text = text(
        "支持 JSON 数组（priority, prompt, model, executor, reasoning_effort, task_split_mode）\n或 CSV/TSV（首行需包含 priority, prompt，建议同时包含 model, executor, reasoning_effort, task_split_mode）",
    )
    .size(12)
    .style(|theme: &Theme| iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text.scale_alpha(0.8)),
    });

    let prompt_tabs = row![
        tab_button(
            "JSON",
            app.task_board_import_prompt_format == crate::app::task::TaskImportPromptFormat::Json,
            crate::app::task::TaskImportPromptFormat::Json
        ),
        tab_button(
            "CSV",
            app.task_board_import_prompt_format == crate::app::task::TaskImportPromptFormat::Csv,
            crate::app::task::TaskImportPromptFormat::Csv
        ),
        tab_button(
            "TSV",
            app.task_board_import_prompt_format == crate::app::task::TaskImportPromptFormat::Tsv,
            crate::app::task::TaskImportPromptFormat::Tsv
        ),
        Space::new().width(Length::Fill),
        button(text("复制").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::CopyImportPromptTemplate))
            .padding([5, 10])
            .style(button_style_secondary),
        button(text(if app.task_board_import_prompt_collapsed { "展开" } else { "折叠" }).size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportPromptCollapsed))
            .padding([5, 10])
            .style(button_style_secondary)
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let prompt_template_text = text(prompt_template)
        .size(12)
        .wrapping(iced::widget::text::Wrapping::Word)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().background.base.text.scale_alpha(0.82)),
        });

    let prompt_template_block: Element<'_, Message> = if app.task_board_import_prompt_collapsed {
        container(text("已折叠，点击“展开”查看模板").size(12))
            .padding([8, 10])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(p.background.weak.color.scale_alpha(0.24))),
                    border: Border {
                        radius: 14.0.into(),
                        width: 1.0,
                        color: p.background.strong.color.scale_alpha(0.42),
                    },
                    ..Default::default()
                }
            })
            .into()
    } else {
        container(prompt_template_text)
            .padding([8, 10])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(p.background.weak.color.scale_alpha(0.20))),
                    border: Border {
                        radius: 14.0.into(),
                        width: 1.0,
                        color: p.background.strong.color.scale_alpha(0.42),
                    },
                    ..Default::default()
                }
            })
            .into()
    };

    column![
        input_label("批量导入内容"),
        import_tools,
        input_label("大模型提示词模板"),
        prompt_tabs,
        prompt_template_block,
        import_editor,
        help_text
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn build_clear_prompt_checkbox(app: &App) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_clear_prompt_after_create)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleClearPromptAfterCreate(enabled))
            })
            .style(settings_checkbox_style),
        text("创建后自动清空提示词").size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn build_close_after_create_checkbox(app: &App) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_close_after_create)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleCloseAfterCreate(enabled))
            })
            .style(settings_checkbox_style),
        text("创建后关闭").size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn build_close_after_edit_checkbox(app: &App) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_close_after_edit)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleCloseAfterEdit(enabled))
            })
            .style(settings_checkbox_style),
        text("编辑后关闭").size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn build_create_options_row(app: &App) -> iced::Element<'_, Message> {
    section_block(
        "创建行为",
        "控制创建完成后的默认收尾动作。",
        row![build_close_after_create_checkbox(app), build_clear_prompt_checkbox(app),]
            .spacing(16)
            .align_y(Alignment::Center),
    )
}

fn build_edit_options_row(app: &App) -> iced::Element<'_, Message> {
    section_block(
        "编辑行为",
        "保存成功后的面板处理策略。",
        row![build_close_after_edit_checkbox(app)].spacing(16).align_y(Alignment::Center),
    )
}

fn build_action_buttons(app: &App, is_edit_mode: bool) -> iced::Element<'static, Message> {
    let (submit_text, submit_msg) = if is_edit_mode {
        if app.task_board_edit_submit_success {
            ("保存成功", TaskBoardMessage::SaveEditingTask)
        } else {
            ("保存修改", TaskBoardMessage::SaveEditingTask)
        }
    } else if app.task_board_is_import_mode {
        ("导入任务", TaskBoardMessage::ImportTasksSubmitted)
    } else if app.task_board_create_submit_success {
        ("保存成功", TaskBoardMessage::CreateTaskSubmitted)
    } else {
        ("创建任务", TaskBoardMessage::CreateTaskSubmitted)
    };
    let submit_style = if is_edit_mode {
        if app.task_board_edit_submit_success { button_style_success } else { button_style_primary }
    } else if app.task_board_create_submit_success {
        button_style_success
    } else {
        button_style_primary
    };

    row![
        button(text("取消").size(14))
            .on_press(Message::TaskBoard(if is_edit_mode {
                TaskBoardMessage::CloseTaskLogs
            } else {
                TaskBoardMessage::CreateTaskCancelled
            }))
            .padding([10, 20])
            .style(button_style_secondary),
        Space::new().width(Length::Fill),
        button(text(submit_text).size(14))
            .on_press(Message::TaskBoard(submit_msg))
            .padding([10, 20])
            .style(submit_style),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

fn build_edit_mode_content<'a>(
    app: &'a App,
    task: &'a crate::app::task::Task,
    mut main_col: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    if !task.subtasks.is_empty() {
        let subtasks_title = text("子任务")
            .size(13)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() });
        main_col = main_col
            .push(Space::new().height(12.0))
            .push(subtasks_title)
            .push(Space::new().height(8.0));

        let (subtasks_col, add_section) = build_edit_mode_subtasks(app, task);
        main_col = main_col.push(subtasks_col).push(Space::new().height(8.0));
        main_col = main_col.push(add_section);
    } else {
        let (_, add_section) = build_edit_mode_subtasks(app, task);
        main_col = main_col.push(Space::new().height(12.0)).push(add_section);
    }

    main_col = main_col
        .push(Space::new().height(16.0))
        .push(build_edit_options_row(app))
        .push(Space::new().height(8.0))
        .push(build_action_buttons(app, true));

    main_col
}

fn build_draft_mode_content<'a>(
    app: &'a App,
    mut main_col: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    if app.task_board_is_import_mode {
        main_col = main_col.push(Space::new().height(16.0)).push(build_action_buttons(app, false));
    } else {
        main_col = main_col
            .push(Space::new().height(12.0))
            .push(build_create_options_row(app))
            .push(Space::new().height(8.0))
            .push(build_draft_mode_subtasks(app))
            .push(Space::new().height(16.0))
            .push(build_action_buttons(app, false));
    }
    main_col
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
