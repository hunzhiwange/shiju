//! 任务看板视图模块，负责看板列、拖拽预览和整体页面组织。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{
    Space, button, checkbox, column, container, row, scrollable, svg, text, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::message::TaskBoardMessage;
use crate::app::task::{Task, TaskStatus};
use crate::app::{App, Message};

use super::common::{
    auto_icon, button_style_danger, button_style_primary, button_style_secondary,
    button_style_success, provider_logo_handle,
};
use super::modals::build_context_menu;
use super::panel::{
    build_bulk_agent_selector, build_bulk_executor_selector, build_bulk_model_selector,
    build_bulk_reasoning_effort_selector, build_bulk_task_split_selector,
};

const CARD_CONTENT_PREVIEW_MAX_CHARS: usize = 180;
const TASK_BOARD_SCROLLBAR_WIDTH: f32 = 4.0;
const TASK_BOARD_SCROLLBAR_GUTTER: f32 = 7.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct KanbanColumn {
    status: TaskStatus,
    label: String,
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

fn task_card_preview(task: &Task) -> String {
    let content =
        if task.prompt.trim().is_empty() { "（无内容）" } else { task.prompt.as_str() };
    let normalized = content.replace(['\n', '\r'], " ");
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    let total = compact.chars().count();
    if total <= CARD_CONTENT_PREVIEW_MAX_CHARS {
        compact
    } else {
        let mut text = compact.chars().take(CARD_CONTENT_PREVIEW_MAX_CHARS).collect::<String>();
        text.push('…');
        text
    }
}

fn task_status_tag_colors(status: TaskStatus) -> (Color, Color) {
    match status {
        TaskStatus::Pool => (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246)),
        TaskStatus::Pending => (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254)),
        TaskStatus::Planning => (Color::from_rgb8(79, 70, 229), Color::from_rgb8(224, 231, 255)),
        TaskStatus::Running => (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255)),
        TaskStatus::Failed => (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226)),
        TaskStatus::Paused => (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195)),
        TaskStatus::CodeComplete => {
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229))
        }
        TaskStatus::CodeReview => (Color::from_rgb8(217, 119, 6), Color::from_rgb8(254, 243, 199)),
        TaskStatus::PrSubmitted => (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254)),
        TaskStatus::Completed => (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231)),
        TaskStatus::Archived => (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249)),
        // 自定义状态暂用中性灰，待工作流迁移任务接管自定义列样式。
        TaskStatus::Custom(_) => (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246)),
    }
}

fn bulk_actions_supported(status: TaskStatus) -> bool {
    status != TaskStatus::Archived
}

fn sort_task_cards(tasks: &mut [&Task]) {
    tasks.sort_by(|a, b| {
        b.created_at_ms
            .cmp(&a.created_at_ms)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn kanban_columns(app: &App) -> Vec<KanbanColumn> {
    let workflow_id = app.task_board_settings.workflow_id.as_str();
    TaskStatus::all_for_workflow_id(workflow_id)
        .into_iter()
        .filter(|status| *status != TaskStatus::Archived)
        .map(|status| KanbanColumn { status, label: status.label_for_workflow_id(workflow_id) })
        .collect()
}

fn task_status_label(app: &App, status: TaskStatus) -> String {
    status.label_for_workflow_id(&app.task_board_settings.workflow_id)
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_kanban_board<'a>(app: &'a App) -> Element<'a, Message> {
    let now_ms = now_ms();
    let mut columns_row = row![].spacing(12).width(Length::Fill).height(Length::Fill);

    for column in kanban_columns(app) {
        let col = build_status_column(app, column.status, now_ms);
        columns_row = columns_row.push(col);
    }

    scrollable(columns_row)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn build_status_column<'a>(app: &'a App, status: TaskStatus, now_ms: u64) -> Element<'a, Message> {
    let mut tasks: Vec<&Task> = app
        .task_board_tasks
        .iter()
        .filter(|t| t.status == status && !t.deleted && !t.archived)
        .collect();
    sort_task_cards(&mut tasks);

    let count = tasks.len();
    let selected_count =
        tasks.iter().filter(|task| app.task_board_selected_tasks.contains(&task.id)).count();
    let bulk_actions_visible = bulk_actions_supported(status);
    let bulk_mode_active = app.task_board_bulk_active_status == Some(status);

    let bulk_toggle_button: Element<'a, Message> = if bulk_actions_visible {
        let icon_color = move |theme: &Theme, _| svg::Style {
            color: Some(if bulk_mode_active {
                Color::WHITE
            } else {
                theme.palette().text.scale_alpha(0.75)
            }),
        };
        button(
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Sliders))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(icon_color),
        )
        .padding(6)
        .style(move |theme: &Theme, button_status| {
            if bulk_mode_active {
                button_style_primary(theme, button_status)
            } else {
                button_style_secondary(theme, button_status)
            }
        })
        .on_press(Message::TaskBoard(TaskBoardMessage::ToggleBulkSelectionMode(status)))
        .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    let selected_summary: Element<'a, Message> = if bulk_mode_active {
        text(format!("已选 {}", selected_count))
            .size(10)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.68)),
            })
            .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    let header_top = row![
        text(task_status_label(app, status))
            .size(13)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        container(text(count.to_string()).size(11)).padding([2, 6]).style(move |theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(
                    if theme.palette().background.r
                        + theme.palette().background.g
                        + theme.palette().background.b
                        < 1.5
                    {
                        p.background.strong.color.scale_alpha(0.56)
                    } else {
                        p.primary.base.color.scale_alpha(0.10)
                    },
                )),
                text_color: Some(
                    if theme.palette().background.r
                        + theme.palette().background.g
                        + theme.palette().background.b
                        < 1.5
                    {
                        p.background.base.text
                    } else {
                        p.primary.base.color
                    },
                ),
                border: Border {
                    radius: 999.0.into(),
                    width: 1.0,
                    color: p.background.strong.color.scale_alpha(0.46),
                },
                ..Default::default()
            }
        }),
        Space::new().width(Length::Fill),
        row![selected_summary, bulk_toggle_button].spacing(6).align_y(Alignment::Center),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut header = column![header_top].spacing(0).width(Length::Fill);

    if bulk_mode_active {
        let selection_actions = row![
            button(text("全选").size(10))
                .padding([4, 8])
                .style(button_style_secondary)
                .on_press(Message::TaskBoard(TaskBoardMessage::SelectAllTasksInStatus(status))),
            button(text("反选").size(10)).padding([4, 8]).style(button_style_secondary).on_press(
                Message::TaskBoard(TaskBoardMessage::InvertTaskSelectionInStatus(status))
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        header = header.push(Space::new().height(6.0)).push(selection_actions);
    }

    if bulk_mode_active && selected_count > 0 {
        let priority_actions = row![
            text_input("优先级", &app.task_board_bulk_priority_input)
                .on_input(|value| Message::TaskBoard(TaskBoardMessage::UpdateBulkPriorityInput(
                    value
                )))
                .padding([5, 8])
                .size(11),
            button(text("应用").size(10))
                .padding([5, 8])
                .style(button_style_success)
                .on_press(Message::TaskBoard(TaskBoardMessage::BulkSetPriorityInStatus(status))),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let model_actions = column![
            build_bulk_model_selector(app),
            row![
                button(text("应用").size(10))
                    .padding([5, 8])
                    .style(button_style_success)
                    .on_press(Message::TaskBoard(TaskBoardMessage::BulkSetModelInStatus(status))),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        ]
        .spacing(6);

        let agent_actions = column![
            build_bulk_agent_selector(app),
            row![
                button(text("应用").size(10))
                    .padding([5, 8])
                    .style(button_style_success)
                    .on_press(Message::TaskBoard(TaskBoardMessage::BulkSetAgentInStatus(status))),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        ]
        .spacing(6);

        let executor_rows = column![
            build_bulk_executor_selector(app),
            row![
                button(text("应用").size(10)).padding([5, 8]).style(button_style_success).on_press(
                    Message::TaskBoard(TaskBoardMessage::BulkSetExecutorInStatus {
                        status,
                        executor: app.task_board_bulk_acp_agent.clone(),
                    })
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        ]
        .spacing(6)
        .width(Length::Fill);

        let reasoning_effort_rows = build_bulk_reasoning_effort_selector(app).map(|selector| {
            column![
                selector,
                row![
                    button(text("应用").size(10))
                        .padding([5, 8])
                        .style(button_style_success)
                        .on_press(Message::TaskBoard(
                            TaskBoardMessage::BulkSetReasoningEffortInStatus {
                                status,
                                reasoning_effort: app.task_board_bulk_reasoning_effort,
                            }
                        )),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
            ]
            .spacing(6)
            .width(Length::Fill)
        });

        let task_split_rows = column![
            build_bulk_task_split_selector(app),
            row![
                button(text("应用").size(10)).padding([5, 8]).style(button_style_success).on_press(
                    Message::TaskBoard(TaskBoardMessage::BulkSetTaskSplitModeInStatus {
                        status,
                        task_split_mode: app.task_board_bulk_task_split_mode,
                    })
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        ]
        .spacing(6)
        .width(Length::Fill);

        header = header
            .push(Space::new().height(8.0))
            .push(text(format!("已选 {} 项", selected_count)).size(10).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text.scale_alpha(0.72)),
                }
            }))
            .push(Space::new().height(6.0))
            .push(text("批量优先级").size(10))
            .push(Space::new().height(4.0))
            .push(priority_actions)
            .push(Space::new().height(6.0))
            .push(text("批量模型").size(10))
            .push(Space::new().height(4.0))
            .push(model_actions)
            .push(Space::new().height(6.0))
            .push(text("批量代理").size(10))
            .push(Space::new().height(4.0))
            .push(agent_actions)
            .push(Space::new().height(6.0))
            .push(text("批量 ACP 智能体").size(10))
            .push(Space::new().height(4.0))
            .push(executor_rows)
            .push(Space::new().height(6.0))
            .push(text("批量推理强度").size(10))
            .push(Space::new().height(4.0))
            .extend(reasoning_effort_rows.map(Into::into))
            .push(Space::new().height(6.0))
            .push(text("批量任务拆分").size(10))
            .push(Space::new().height(4.0))
            .push(task_split_rows)
            .push(Space::new().height(6.0));

        if status == TaskStatus::PrSubmitted {
            let merge_row = row![
                button(text("批量合并").size(10))
                    .padding([5, 8])
                    .style(button_style_success)
                    .on_press(Message::TaskBoard(TaskBoardMessage::BulkMergeTasksInStatus(status))),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            header = header
                .push(text("手动合并队列").size(10))
                .push(Space::new().height(4.0))
                .push(merge_row)
                .push(Space::new().height(6.0));
        }

        let action_row = row![
            button(text("批量归档").size(10))
                .padding([4, 8])
                .style(button_style_primary)
                .on_press(Message::TaskBoard(TaskBoardMessage::BulkArchiveTasksInStatus(status))),
            button(text("批量删除").size(10))
                .padding([4, 8])
                .style(button_style_danger)
                .on_press(Message::TaskBoard(TaskBoardMessage::BulkDeleteTasksInStatus(status))),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        header = header.push(Space::new().height(6.0)).push(action_row);

        let move_targets = TaskStatus::all_for_workflow_id(&app.task_board_settings.workflow_id)
            .into_iter()
            .filter(|candidate| *candidate != status && *candidate != TaskStatus::Archived)
            .collect::<Vec<_>>();
        let mut move_rows = column![].spacing(6).width(Length::Fill);
        for chunk in move_targets.chunks(2) {
            let mut move_row = row![].spacing(6).width(Length::Fill);
            for target in chunk {
                move_row = move_row.push(
                    button(text(format!("移到{}", task_status_label(app, *target))).size(10))
                        .padding([4, 6])
                        .style(button_style_secondary)
                        .on_press(Message::TaskBoard(TaskBoardMessage::BulkMoveTasksInStatus {
                            from_status: status,
                            to_status: *target,
                        })),
                );
            }
            move_rows = move_rows.push(move_row);
        }
        header = header
            .push(Space::new().height(6.0))
            .push(text("批量移动").size(10).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.68)),
            }))
            .push(Space::new().height(4.0))
            .push(move_rows);
    } else if bulk_mode_active {
        header = header.push(Space::new().height(8.0)).push(
            text("勾选任务后可批量设置模型、优先级和 ACP 智能体").size(10).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text.scale_alpha(0.68)),
                },
            ),
        );
    }

    let mut tasks_col = column![].spacing(0);

    let dragging_task_id =
        app.task_board_dragging.as_ref().map(|(id, _): &(String, TaskStatus)| id.as_str());
    let cursor_position = app.cursor_position;
    let context_menu = app.task_board_context_menu.clone();
    let is_dragging_active = app.task_board_dragging.is_some();
    let mut insert_index = 0usize;
    tasks_col = tasks_col.push(build_drop_slot(status, insert_index, is_dragging_active));
    for task in &tasks {
        let is_dragging = dragging_task_id == Some(task.id.as_str());
        let card = build_task_card(
            app,
            task,
            status,
            is_dragging,
            is_dragging_active,
            cursor_position,
            context_menu.clone(),
            now_ms,
        );
        tasks_col = tasks_col.push(card);
        insert_index = insert_index.saturating_add(1);
        tasks_col = tasks_col.push(build_drop_slot(status, insert_index, is_dragging_active));
    }

    let right_inset =
        if app.task_board_column_has_vertical_scrollbar.get(&status).copied().unwrap_or(false) {
            TASK_BOARD_SCROLLBAR_GUTTER
        } else {
            0.0
        };
    let content = container(
        column![header, Space::new().height(10.0), tasks_col].spacing(0).width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(iced::Padding { top: 0.0, right: right_inset, bottom: 0.0, left: 0.0 });

    let is_drop_target = app.task_board_dragging.is_some();

    let container_style = move |theme: &Theme| {
        let base = theme.palette().background;
        let is_dark = base.r + base.g + base.b < 1.5;
        let bg = if is_dark {
            theme.extended_palette().background.base.color.scale_alpha(0.90)
        } else {
            Color::from_rgba8(255, 255, 255, 0.86)
        };
        container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                width: if is_drop_target { 2.0 } else { 1.0 },
                color: if is_drop_target {
                    theme.palette().primary.scale_alpha(0.86)
                } else {
                    theme.extended_palette().background.strong.color.scale_alpha(0.72)
                },
                radius: 20.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                offset: iced::Vector::new(0.0, 12.0),
                blur_radius: 26.0,
            },
            ..Default::default()
        }
    };

    let col_container = container(
        scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(TASK_BOARD_SCROLLBAR_WIDTH)
                    .scroller_width(TASK_BOARD_SCROLLBAR_WIDTH),
            ))
            .on_scroll(move |viewport| {
                Message::TaskBoard(TaskBoardMessage::ColumnScrollChanged {
                    status,
                    has_vertical_scrollbar: viewport.content_bounds().height
                        > viewport.bounds().height,
                })
            })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(240.0))
    .height(Length::Fill)
    .padding(12)
    .style(container_style);

    let mouse_area = iced::widget::mouse_area(col_container);

    if is_drop_target {
        mouse_area
            .on_release(Message::TaskBoard(TaskBoardMessage::DropOnStatus {
                to_status: status,
                insert_index: None,
            }))
            .into()
    } else {
        mouse_area.into()
    }
}

fn build_drop_slot<'a>(
    status: TaskStatus,
    insert_index: usize,
    is_dragging_active: bool,
) -> Element<'a, Message> {
    let slot = container(Space::new().height(8.0)).width(Length::Fill);
    let mouse_area = iced::widget::mouse_area(slot);
    if is_dragging_active {
        mouse_area
            .on_release(Message::TaskBoard(TaskBoardMessage::DropOnStatus {
                to_status: status,
                insert_index: Some(insert_index),
            }))
            .into()
    } else {
        mouse_area.into()
    }
}

fn build_task_card<'a>(
    app: &'a App,
    task: &'a Task,
    status: TaskStatus,
    is_dragging: bool,
    is_dragging_active: bool,
    cursor_position: iced::Point,
    context_menu: Option<(String, f32, f32)>,
    now_ms: u64,
) -> Element<'a, Message> {
    let is_selected = app.task_board_selected_tasks.contains(&task.id);
    let bulk_mode_active = app.task_board_bulk_active_status == Some(status);
    let priority_color = if task.priority <= 100 {
        Color::from_rgb8(239, 68, 68)
    } else if task.priority <= 500 {
        Color::from_rgb8(245, 158, 11)
    } else {
        Color::from_rgb8(107, 114, 128)
    };

    let priority_badge = container(text(format!("P{}", task.priority)).size(9))
        .padding([1, 4])
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(priority_color.scale_alpha(0.2))),
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Some(priority_color),
            ..Default::default()
        });

    let content = task_card_preview(task);
    let (status_color, status_bg_color) = task_status_tag_colors(task.status);
    let status_text = if task.status == TaskStatus::Running {
        format!("{} {}", task_status_label(app, task.status), running_dots(now_ms))
    } else {
        task_status_label(app, task.status)
    };
    let status_badge =
        container(text(status_text).size(9)).padding([1, 6]).style(move |_theme: &Theme| {
            container::Style {
                background: Some(Background::Color(status_bg_color)),
                border: Border { radius: 999.0.into(), ..Default::default() },
                text_color: Some(status_color),
                ..Default::default()
            }
        });
    let task_id_text = text(&task.id).size(9).style(|theme: &Theme| iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text),
    });
    let selection_section: Element<'a, Message> = if bulk_mode_active {
        checkbox(is_selected)
            .on_toggle({
                let task_id = task.id.clone();
                move |selected| {
                    Message::TaskBoard(TaskBoardMessage::ToggleTaskSelection {
                        task_id: task_id.clone(),
                        selected,
                    })
                }
            })
            .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };
    let header_row = row![
        row![selection_section, task_id_text, status_badge].spacing(6).align_y(Alignment::Center),
        Space::new().width(Length::Fill),
        priority_badge
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let title_text = text(content).size(12).wrapping(iced::widget::text::Wrapping::Word);
    let title_row = row![container(title_text).width(Length::Fill)].width(Length::Fill);

    let auto_model = task.model == "auto";
    let model_icon_svg = if auto_model {
        svg::Svg::<iced::Theme>::new(auto_icon())
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    } else {
        let provider_id = if task.model.contains('/') {
            task.model.split('/').next().unwrap_or("agent").to_string()
        } else {
            "agent".to_string()
        };
        svg::Svg::<iced::Theme>::new(provider_logo_handle(&provider_id))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    };

    let model_row = row![
        model_icon_svg,
        text(&task.model).size(10).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().background.base.text)
        }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let subtask_count = task.subtasks.len();
    let subtask_info = if subtask_count > 0 {
        let completed_count = task.subtasks.iter().filter(|s| s.completed).count();
        Some(
            row![
                text("📋").size(10),
                text(format!("{}/{}", completed_count, subtask_count)).size(10).style(
                    |theme: &Theme| {
                        iced::widget::text::Style {
                            color: Some(theme.extended_palette().background.base.text),
                        }
                    }
                ),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
    } else {
        None
    };

    let mut content_col = column![
        header_row,
        Space::new().height(4.0),
        title_row,
        Space::new().height(6.0),
        model_row,
    ]
    .spacing(2)
    .width(Length::Fill);

    if task.retry_count > 0 {
        let retry_row = row![
            text("重试次数").size(10),
            text(task.retry_count.to_string()).size(10).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text),
                }
            }),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        content_col = content_col.push(retry_row);
    }

    if let Some(duration_ms) = task.display_execution_duration_ms(now_ms) {
        let duration_text = if task.status == TaskStatus::Running {
            format!("执行中 {}", format_duration_ms(duration_ms))
        } else {
            format!("已执行 {}", format_duration_ms(duration_ms))
        };
        let duration_row = row![
            text("⏱").size(10),
            text(duration_text).size(10).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text),
            }),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        content_col = content_col.push(duration_row);
    }

    if task.status == TaskStatus::Failed {
        let failed_reason = task.last_error.as_deref().unwrap_or("未知错误");
        let reason_text = text(format!("失败原因: {}", failed_reason))
            .size(10)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().danger),
            });
        content_col = content_col.push(container(reason_text).width(Length::Fill));
    }
    if task.status == TaskStatus::Paused {
        let paused_reason = task.pause_reason.as_deref().unwrap_or("人工暂停");
        let reason_text = text(format!("暂停原因: {}", paused_reason))
            .size(10)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().warning.base.color),
            });
        content_col = content_col.push(container(reason_text).width(Length::Fill));
    }

    if let Some(subtask_row) = subtask_info {
        content_col = content_col.push(subtask_row);
    }

    let card =
        container(content_col).width(Length::Fill).padding([12, 12]).style(move |theme: &Theme| {
            let bg = if is_dragging {
                theme.palette().primary.scale_alpha(0.14)
            } else if is_selected {
                theme.palette().primary.scale_alpha(0.08)
            } else if theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5
            {
                theme.extended_palette().background.weak.color.scale_alpha(0.18)
            } else {
                Color::WHITE.scale_alpha(0.82)
            };
            let border_color = if is_dragging {
                theme.palette().primary.scale_alpha(0.92)
            } else if is_selected {
                theme.palette().primary.scale_alpha(0.70)
            } else {
                theme.extended_palette().background.strong.color.scale_alpha(0.56)
            };
            container::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    width: if is_dragging || is_selected { 2.0 } else { 1.0 },
                    color: border_color,
                    radius: 16.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if is_dragging || is_selected {
                        0.14
                    } else {
                        0.06
                    }),
                    offset: iced::Vector::new(0.0, 10.0),
                    blur_radius: 22.0,
                },
                ..Default::default()
            }
        });

    let mut mouse_area = iced::widget::mouse_area(card).on_press(Message::TaskBoard(
        TaskBoardMessage::DragPending {
            task_id: task.id.clone(),
            from_status: status,
            press_position: cursor_position,
        },
    ));
    if !is_dragging_active {
        mouse_area = mouse_area.on_release(Message::TaskBoard(TaskBoardMessage::CardReleased {
            task_id: task.id.clone(),
        }));
    }

    let task_id_for_menu = task.id.clone();
    let right_click = Element::new(RightClickArea::new(
        mouse_area.into(),
        Box::new(move |pos| {
            Message::TaskBoard(TaskBoardMessage::ContextMenuOpened {
                task_id: task_id_for_menu.clone(),
                x: pos.x,
                y: pos.y,
            })
        }),
    ));

    if let Some((open_id, x, y)) = context_menu
        && open_id == task.id
    {
        return PointBelowOverlay::new(right_click, build_context_menu(task.clone()))
            .show(true)
            .anchor(iced::Point::new(x, y))
            .gap(2.0)
            .on_close(Message::TaskBoard(TaskBoardMessage::ContextMenuClosed))
            .into();
    }

    right_click
}

#[cfg(test)]
#[path = "board_tests.rs"]
mod board_tests;
