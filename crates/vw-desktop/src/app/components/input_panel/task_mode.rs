//! 任务模式表单模块
//!
//! 本模块提供任务模式相关的 UI 组件，用于构建和管理任务执行表单界面。
//!
//! # 主要功能
//!
//! - 任务优先级配置：允许用户设置任务的执行优先级
//! - ACP 智能体选择：支持选择不同的 ACP 智能体
//! - 子任务管理：提供完整的子任务增删改查和排序功能
//!
//! # 架构位置
//!
//! 该模块属于输入面板（input_panel）组件的一部分，负责渲染任务模式下的配置表单。

use std::fmt;

use iced::widget::scrollable;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{
    Space, button, column, container, pick_list, row, text, text_editor, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::{acp_agent_icon, icon_svg};
use crate::app::components::input_panel::reasoning_selector::{
    reasoning_effort_label, reasoning_effort_options,
};
use crate::app::components::input_panel::styles::{
    popover_style, selectable_list_button_style, selector_chevron_color, selector_label_font,
    selector_pill_button_style, selector_text_color, square_icon_button_style,
    subtask_editor_style,
};
use crate::app::components::overlays::AboveOverlay;
use crate::app::state::SessionRuntimeState;
use crate::app::task::TaskSplitMode;
use crate::app::{App, Message, message};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

const ACP_SELECTOR_MAX_HEIGHT: f32 = 240.0;
const ACP_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
const ACP_SELECTOR_LIST_RIGHT_PADDING: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TaskModeReasoningEffortOption(pub(super) Option<ReasoningEffortDto>);

impl fmt::Display for TaskModeReasoningEffortOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(reasoning_effort_label(self.0))
    }
}

pub(super) fn task_mode_reasoning_effort_options(
    agent: Option<&str>,
) -> Vec<Option<ReasoningEffortDto>> {
    reasoning_effort_options(agent.unwrap_or_default()).to_vec()
}

/// 构建任务模式配置表单
///
/// 该函数创建一个包含任务优先级、执行器选择和子任务管理的完整表单界面。
/// 当任务模式未启用时，返回一个空白占位符。
///
/// # 参数
///
/// - `app`: 应用程序主状态的引用，用于访问全局状态（如弹出框显示状态）
/// - `runtime`: 可选的会话运行时状态引用，用于访问子任务编辑器内容
/// - `task_mode_enabled`: 任务模式是否启用的标志
/// - `task_mode_priority`: 当前任务的优先级字符串（数字越小优先级越高）
/// - `task_mode_model`: 当前任务模式使用的模型标识符
/// - `task_mode_executor`: 当前选中的 ACP 智能体
/// - `task_mode_subtasks`: 子任务列表的切片引用
///
/// # 返回值
///
/// 返回一个 Iced UI 元素，包含完整的任务模式表单界面
///
/// # 示例
///
/// ```ignore
/// let form = task_mode_form(
///     &app,
///     Some(&runtime),
///     true,
///     "10",
///     Some("claude".to_string()),
///     &["子任务1".to_string(), "子任务2".to_string()],
/// );
/// ```
pub fn task_mode_form<'a>(
    app: &'a App,
    runtime: Option<&'a SessionRuntimeState>,
    task_mode_enabled: bool,
    task_mode_priority: &str,
    task_mode_model: &str,
    task_mode_task_split_mode: TaskSplitMode,
    task_mode_executor: Option<String>,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
    task_mode_subtasks: &'a [String],
) -> Element<'a, Message> {
    // 如果任务模式未启用，返回空白占位符
    if !task_mode_enabled {
        return Space::new().into();
    }

    // 构建优先级输入框
    // 提示用户输入数字，数字越小代表优先级越高
    let priority_input = text_input("数字越小越优先", task_mode_priority)
        .on_input(|v| Message::Chat(message::ChatMessage::TaskModePriorityChanged(v)))
        .padding([8, 10])
        .size(12)
        .width(Length::Fixed(180.0));

    // 优先级行布局：左侧标签，右侧输入框
    let priority_row =
        row![text("优先级").size(12), Space::new().width(Length::Fill), priority_input]
            .align_y(Alignment::Center)
            .spacing(10);

    let model_input = text_input("auto / provider/model / 自定义模型", task_mode_model)
        .on_input(|v| Message::Chat(message::ChatMessage::TaskModeModelChanged(v)))
        .padding([8, 10])
        .size(12)
        .width(Length::Fixed(180.0));

    let model_row = row![text("大模型").size(12), Space::new().width(Length::Fill), model_input]
        .align_y(Alignment::Center)
        .spacing(10);

    let task_split_field =
        pick_list(TaskSplitMode::all().to_vec(), Some(task_mode_task_split_mode), |mode| {
            Message::Chat(message::ChatMessage::TaskModeTaskSplitModeChanged(mode))
        })
        .text_size(12)
        .padding([6, 10])
        .width(Length::Fixed(180.0));

    let task_split_row =
        row![text("任务拆分").size(12), Space::new().width(Length::Fill), task_split_field]
            .align_y(Alignment::Center)
            .spacing(10);

    // 构建子任务列表容器
    // 遍历所有子任务，为每个子任务创建编辑卡片
    let mut subtask_rows = column![].spacing(6);
    for (idx, subtask) in task_mode_subtasks.iter().enumerate() {
        // 创建子任务序号徽章
        // 显示当前子任务的序号（从1开始计数）
        let index_badge = container(text(format!("{}", idx + 1)).size(11))
            .width(Length::Fixed(20.0))
            .height(Length::Fixed(20.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(p.background.weak.color)),
                    border: Border {
                        width: 1.0,
                        color: p.background.strong.color,
                        radius: 999.0.into(),
                    },
                    ..Default::default()
                }
            });

        // 构建子任务输入框
        // 优先使用富文本编辑器（如果运行时状态中存在该子任务的编辑器实例）
        // 否则使用普通文本输入框
        let subtask_input: Element<'_, Message> = if let Some(subtask_editor) =
            runtime.and_then(|r| r.task_mode_subtask_editors.get(idx))
        {
            // 计算富文本编辑器高度：根据文本行数动态调整（1-3行）
            let subtask_line_count = subtask_editor.text().split('\n').count().clamp(1, 3) as f32;
            let subtask_editor_height = subtask_line_count * 18.0 + 8.0;

            // 富文本编辑器模式：支持多行文本编辑
            container(
                text_editor(subtask_editor)
                    .placeholder("输入子任务内容...")
                    .on_action(move |action| {
                        Message::Chat(message::ChatMessage::TaskModeSubtaskEditorAction {
                            index: idx,
                            action,
                        })
                    })
                    .size(12)
                    .padding(iced::Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
                    .height(subtask_editor_height)
                    .style(subtask_editor_style),
            )
            .width(Length::Fill)
            .into()
        } else {
            // 普通文本输入模式：单行文本编辑
            text_input("输入子任务内容...", subtask)
                .on_input(move |v| {
                    Message::Chat(message::ChatMessage::TaskModeSubtaskChanged {
                        index: idx,
                        value: v,
                    })
                })
                .padding([6, 8])
                .size(12)
                .width(Length::Fill)
                .into()
        };

        // 上移按钮：如果不是第一个子任务，则显示可点击的上移按钮
        // 否则显示禁用状态的上移按钮（无背景色）
        let up_btn: Element<'_, Message> = if idx > 0 {
            button(text("↑").size(10))
                .on_press(Message::Chat(message::ChatMessage::TaskModeMoveSubtaskUp(idx)))
                .padding([2, 6])
                .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
                .into()
        } else {
            button(text("↑").size(10))
                .padding([2, 6])
                .style(|theme: &Theme, status| {
                    let mut style = square_icon_button_style(theme, status, false);
                    style.background = None;
                    style
                })
                .into()
        };

        // 下移按钮：如果不是最后一个子任务，则显示可点击的下移按钮
        // 否则显示禁用状态的下移按钮（无背景色）
        let down_btn: Element<'_, Message> = if idx + 1 < task_mode_subtasks.len() {
            button(text("↓").size(10))
                .on_press(Message::Chat(message::ChatMessage::TaskModeMoveSubtaskDown(idx)))
                .padding([2, 6])
                .style(|theme: &Theme, status| square_icon_button_style(theme, status, true))
                .into()
        } else {
            button(text("↓").size(10))
                .padding([2, 6])
                .style(|theme: &Theme, status| {
                    let mut style = square_icon_button_style(theme, status, false);
                    style.background = None;
                    style
                })
                .into()
        };

        // 删除按钮：允许用户删除当前子任务
        let delete_btn = button(text("×").size(10))
            .on_press(Message::Chat(message::ChatMessage::TaskModeRemoveSubtask(idx)))
            .padding([2, 6])
            .style(|theme: &Theme, status| square_icon_button_style(theme, status, true));

        // 组装子任务行：序号徽章 + 输入框 + 上移按钮 + 下移按钮 + 删除按钮
        let subtask_row = row![index_badge, subtask_input, up_btn, down_btn, delete_btn]
            .spacing(6)
            .align_y(Alignment::Center);

        // 将子任务行包装为卡片样式
        let subtask_card =
            container(subtask_row).padding([6, 10]).width(Length::Fill).style(|theme: &Theme| {
                let p = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(p.background.base.color)),
                    border: Border {
                        width: 1.0,
                        color: p.background.strong.color,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            });
        subtask_rows = subtask_rows.push(subtask_card);
    }

    // 新增子任务按钮：允许用户添加新的子任务项
    let add_subtask_btn = button(text("新增子任务").size(11))
        .on_press(Message::Chat(message::ChatMessage::TaskModeAddSubtask))
        .padding([6, 10])
        .style(|theme: &Theme, status| square_icon_button_style(theme, status, true));

    let effective_acp_agent = task_mode_executor.clone();
    let selected_for_default = effective_acp_agent.is_none();
    let selected_for_popover = effective_acp_agent.clone();
    let label = effective_acp_agent.as_deref().unwrap_or("ACP 智能体").to_string();
    let highlight_toggle = app.show_executor_popover || effective_acp_agent.is_some();

    // ACP 智能体切换按钮：显示当前智能体名称和下拉箭头图标
    let executor_toggle = button(
        row![
            acp_agent_icon(effective_acp_agent.as_deref().unwrap_or("ACP 智能体"), 14.0),
            text(label).size(13).font(selector_label_font()).style(move |theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(selector_text_color(theme, highlight_toggle)),
                }
            }),
            icon_svg(Icon::ChevronDown, 14.0).style(move |theme: &Theme, _| svg::Style {
                color: Some(selector_chevron_color(theme, highlight_toggle)),
            })
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(move |theme: &Theme, status| selector_pill_button_style(theme, status, highlight_toggle))
    .padding([6, 12])
    .on_press(Message::View(message::ViewMessage::ToggleExecutorPopover));

    // 构建 ACP 智能体选择弹出框内容
    let executor_pop_content: Element<'_, Message> = {
        let mut executor_list = column![].spacing(4);

        let default_check: Element<'_, Message> = if selected_for_default {
            icon_svg(Icon::Check, 14.0).into()
        } else {
            Space::new().width(Length::Fixed(14.0)).into()
        };

        let default_btn = button(
            row![
                acp_agent_icon("ACP 智能体", 14.0),
                text("ACP 智能体").size(13),
                Space::new().width(Length::Fill),
                default_check
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .width(Length::Fill)
        .style(move |theme: &Theme, status: iced::widget::button::Status| {
            selectable_list_button_style(theme, status, selected_for_default)
        })
        .on_press(Message::Chat(message::ChatMessage::TaskModeExecutorChanged(None)));

        executor_list = executor_list.push(default_btn);

        for agent in app.acp_agents.iter().cloned() {
            let selected = selected_for_popover.as_ref() == Some(&agent);
            let check: Element<'_, Message> = if selected {
                icon_svg(Icon::Check, 14.0).into()
            } else {
                Space::new().width(Length::Fixed(14.0)).into()
            };

            let agent_for_press = agent.clone();

            let select_btn = button(
                row![
                    acp_agent_icon(&agent, 14.0),
                    text(agent).size(13),
                    Space::new().width(Length::Fill),
                    check
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme, status: iced::widget::button::Status| {
                selectable_list_button_style(theme, status, selected)
            })
            .on_press(Message::Chat(message::ChatMessage::TaskModeExecutorChanged(Some(
                agent_for_press,
            ))));

            executor_list = executor_list.push(select_btn);
        }

        // 将 ACP 智能体列表包装为弹出框样式
        container(
            scrollable(container(executor_list).padding(iced::Padding {
                top: 0.0,
                right: ACP_SELECTOR_LIST_RIGHT_PADDING,
                bottom: 0.0,
                left: 0.0,
            }))
            .id(iced::widget::Id::new("input_panel_task_mode_executor_scroll"))
            .direction(Direction::Vertical(
                Scrollbar::new()
                    .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                    .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
            ))
            .height(Length::Fixed(ACP_SELECTOR_MAX_HEIGHT)),
        )
        .style(popover_style)
        .padding(8)
        .width(Length::Fixed(180.0))
        .into()
    };

    // 使用 AboveOverlay 组件包装执行器切换按钮和弹出框
    // 实现点击按钮时显示/隐藏执行器选择列表的交互效果
    let executor_btn: Element<'_, Message> =
        AboveOverlay::new(executor_toggle, executor_pop_content)
            .show(app.show_executor_popover)
            .gap(6.0)
            .on_close(Message::View(message::ViewMessage::CloseExecutorPopover))
            .into();

    // ACP 智能体行布局：左侧标签，右侧智能体选择按钮
    let executor_row =
        row![text("ACP 智能体").size(12), Space::new().width(Length::Fill), executor_btn]
            .align_y(Alignment::Center);

    let reasoning_effort_field = pick_list(
        task_mode_reasoning_effort_options(effective_acp_agent.as_deref())
            .into_iter()
            .map(TaskModeReasoningEffortOption)
            .collect::<Vec<_>>(),
        Some(TaskModeReasoningEffortOption(task_mode_reasoning_effort)),
        |option| Message::Chat(message::ChatMessage::TaskModeReasoningEffortChanged(option.0)),
    )
    .text_size(12)
    .padding([6, 10])
    .width(Length::Fixed(180.0));
    let reasoning_effort_row =
        row![text("推理强度").size(12), Space::new().width(Length::Fill), reasoning_effort_field]
            .align_y(Alignment::Center)
            .spacing(10);

    // 组装完整的表单布局
    // 包含优先级配置、执行器选择、子任务列表和新增按钮
    let form = column![
        priority_row,
        model_row,
        task_split_row,
        executor_row,
        reasoning_effort_row,
        text("子任务").size(12),
        subtask_rows,
        row![add_subtask_btn, Space::new().width(Length::Fill)].align_y(Alignment::Center)
    ]
    .spacing(6)
    .width(Length::Fill);

    // 将表单包装为容器，应用样式（背景、边框、阴影）
    container(form)
        .width(Length::Fill)
        .padding(iced::Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                // 半透明背景
                background: Some(Background::Color(p.background.base.color.scale_alpha(0.92))),
                border: Border {
                    width: 1.0,
                    color: p.background.strong.color,
                    radius: 14.0.into(),
                },
                // 添加阴影效果增强视觉层次
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.06),
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            }
        })
        .into()
}
