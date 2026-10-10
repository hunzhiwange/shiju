//! 任务看板模态窗口组件模块
//!
//! 本模块提供了任务看板中使用的各种模态窗口和弹出菜单的构建函数。
//! 主要包含以下功能：
//!
//! - **新建任务模态窗口**：支持单个任务创建和批量导入两种模式
//! - **任务设置模态窗口**：配置任务执行的相关参数
//! - **上下文菜单**：任务项的右键操作菜单
//!
//! ## 模块结构
//!
//! 所有模态窗口都基于 `iced` UI 框架构建，返回 `Element<Message>` 类型的 UI 组件。
//! 模态窗口的样式遵循应用程序的整体主题设计，包括按钮、输入框、容器等组件。

use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_editor, text_input,
    toggler,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::components::system_settings_common::{
    settings_close_button, settings_modal_card, settings_muted_text_style, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_text_editor_style, settings_text_input_style,
};
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardSettingsModalTab;
use crate::app::task::{Task, TaskSplitMode};
use crate::app::{App, Message};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

use super::common::{button_style_primary, button_style_secondary};
use super::panel::{build_executor_selector, build_model_selector};

/// 构建新建任务模态窗口
///
/// 该函数创建一个完整的任务创建模态窗口，支持两种模式：
/// - **单个任务模式**：通过表单输入单个任务的详细信息
/// - **批量导入模式**：通过 JSON 或 CSV 格式批量导入多个任务
///
/// # 功能特性
///
/// - 模型选择：支持自动模型选择和手动指定模型
/// - ACP 智能体选择：支持默认 ACP 或指定具体 ACP 智能体
/// - 优先级设置：支持 1-99999 的优先级值
/// - 提示词编辑：内置文本编辑器用于编写大模型提示词
/// - 批量导入：支持 JSON 数组和 CSV/TSV 格式的批量任务导入
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，用于获取当前任务草稿状态
///
/// # 返回值
///
/// 返回构建好的模态窗口 UI 元素，类型为 `Element<'_, Message>`
///
/// # 示例
///
/// ```ignore
/// let modal = build_create_task_modal(&app);
/// // 将 modal 添加到 UI 树中进行渲染
/// ```
#[allow(dead_code)]
pub fn build_create_task_modal(app: &App) -> Element<'_, Message> {
    // 创建输入字段标签
    //
    // 生成带有主题适配样式的标签文本组件
    //
    // # 参数
    //
    // * `label` - 标签文本内容
    //
    // # 返回值
    //
    // 返回样式化后的标签 UI 元素
    fn input_label(label: &str) -> Element<'_, Message> {
        text(label)
            .size(12)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text),
            })
            .into()
    }

    // 定义文本输入框的样式
    //
    // 根据当前主题生成文本输入框的视觉样式，包括背景色、边框、图标颜色等
    //
    // # 参数
    //
    // * `theme` - 当前 iced 主题
    // * `_status` - 文本输入框的当前状态（未使用）
    //
    // # 返回值
    //
    // 返回文本输入框的样式配置
    fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
        settings_text_input_style(theme, status)
    }

    // 定义文本编辑器的样式
    //
    // 根据当前主题生成文本编辑器的视觉样式，用于提示词和批量导入内容的编辑
    //
    // # 参数
    //
    // * `theme` - 当前 iced 主题
    // * `_status` - 文本编辑器的当前状态（未使用）
    //
    // # 返回值
    //
    // 返回文本编辑器的样式配置
    fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
        settings_text_editor_style(theme, status)
    }

    // 构建模态窗口标题栏
    // 包含标题文本、填充空间和关闭按钮
    let header = row![
        text("新建任务")
            .size(18)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        Space::new().width(Length::Fill),
        settings_close_button(Message::TaskBoard(TaskBoardMessage::CreateTaskCancelled)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    // 构建优先级输入框
    // 允许用户输入 1-99999 范围的优先级值
    let priority_input = text_input("1-99999，默认999", &app.task_board_draft.priority)
        .on_input(|v| Message::TaskBoard(TaskBoardMessage::UpdateDraftPriority(v)))
        .padding([8, 10])
        .size(14)
        .width(Length::Fill)
        .style(input_style);

    let priority_field = priority_input;
    let model_field = build_model_selector(app, false);
    let executor_field = build_executor_selector(app, false);
    let task_split_field = pick_list(
        TaskSplitMode::all().to_vec(),
        Some(app.task_board_draft.task_split_mode),
        |mode| Message::TaskBoard(TaskBoardMessage::UpdateDraftTaskSplitMode(mode)),
    )
    .placeholder("任务拆分")
    .padding([6, 10])
    .text_size(14)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill);

    // 构建模式切换按钮组：单个任务 / 批量导入
    let mode_toggle = row![
        button(text("单个任务").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(false)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                let p = theme.extended_palette();
                let active = !app.task_board_is_import_mode;
                let bg = if active {
                    Some(Background::Color(p.background.strong.color))
                } else {
                    match status {
                        iced::widget::button::Status::Hovered => {
                            Some(Background::Color(p.background.weak.color.scale_alpha(0.5)))
                        }
                        _ => None,
                    }
                };
                iced::widget::button::Style {
                    background: bg,
                    text_color: theme.palette().text,
                    border: Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }
            }),
        button(text("批量导入").size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(true)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                let p = theme.extended_palette();
                let active = app.task_board_is_import_mode;
                let bg = if active {
                    Some(Background::Color(p.background.strong.color))
                } else {
                    match status {
                        iced::widget::button::Status::Hovered => {
                            Some(Background::Color(p.background.weak.color.scale_alpha(0.5)))
                        }
                        _ => None,
                    }
                };
                iced::widget::button::Style {
                    background: bg,
                    text_color: theme.palette().text,
                    border: Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }
            }),
    ]
    .spacing(8);

    // 根据当前模式构建主体内容
    let body_content: Element<'_, Message> = if app.task_board_is_import_mode {
        // 批量导入模式：显示文本编辑器和帮助文本
        let import_editor = text_editor(&app.task_board_import_editor)
            .placeholder("粘贴 JSON 数组或 CSV 内容...")
            .on_action(|a| Message::TaskBoard(TaskBoardMessage::ImportEditorAction(a)))
            .size(14)
            .padding([8, 10])
            .height(Length::Fixed(300.0))
            .style(editor_style);

        let help_text = text("支持 JSON 数组格式（包含 priority, prompt, model, acp_agent, task_split_mode 字段）\n或 CSV/TSV 格式（首行需包含 priority, prompt 表头）")
            .size(12)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.8)),
            });

        column![input_label("批量导入内容"), import_editor, help_text]
            .spacing(6)
            .width(Length::Fill)
            .into()
    } else {
        // 单个任务模式：显示提示词编辑器和配置字段
        let prompt_editor = text_editor(&app.task_board_prompt_editor)
            .placeholder("输入大模型提示词...")
            .on_action(|a| Message::TaskBoard(TaskBoardMessage::PromptEditorAction(a)))
            .size(14)
            .padding([8, 10])
            .height(Length::Fixed(200.0))
            .style(editor_style);

        let prompt_field =
            column![input_label("大模型提示词"), prompt_editor].spacing(6).width(Length::Fill);

        column![
            prompt_field,
            Space::new().height(12.0),
            // 模型、ACP 智能体和优先级字段并排显示
            row![
                container(model_field).width(Length::FillPortion(1)),
                container(executor_field).width(Length::FillPortion(1)),
                container(task_split_field).width(Length::FillPortion(1)),
                container(priority_field).width(Length::FillPortion(1)),
            ]
            .spacing(12),
        ]
        .spacing(0)
        .into()
    };

    // 根据当前模式确定提交按钮的文本和消息
    let submit_btn_text =
        if app.task_board_is_import_mode { "导入任务" } else { "创建任务" };
    let submit_btn_msg = if app.task_board_is_import_mode {
        Message::TaskBoard(TaskBoardMessage::ImportTasksSubmitted)
    } else {
        Message::TaskBoard(TaskBoardMessage::CreateTaskSubmitted)
    };

    // 构建底部按钮组：取消和提交
    let buttons = row![
        button(text("取消").size(14))
            .on_press(Message::TaskBoard(TaskBoardMessage::CreateTaskCancelled))
            .padding([10, 20])
            .style(button_style_secondary),
        Space::new().width(Length::Fill),
        button(text(submit_btn_text).size(14))
            .on_press(submit_btn_msg)
            .padding([10, 20])
            .style(button_style_primary),
    ]
    .spacing(8)
    .width(Length::Fill);

    // 组装完整的模态窗口内容
    let modal_content = scrollable(
        column![
            header,
            Space::new().height(16.0),
            mode_toggle,
            Space::new().height(16.0),
            body_content,
            Space::new().height(20.0),
            buttons,
        ]
        .spacing(0)
        .width(Length::Fixed(480.0)),
    )
    .height(Length::Fill);

    // 返回带有容器样式的完整模态窗口
    settings_modal_card(modal_content)
        .width(Length::Fill)
        .height(Length::Fixed(480.0))
        .center_x(Length::Fill)
        .into()
}

/// 构建任务设置模态窗口
///
/// 该函数创建一个任务设置模态窗口，用于配置任务看板的全局执行参数。
/// 这些设置会影响所有任务的执行行为
///
/// # 可配置参数
///
/// - **并发数**：同时执行的最大任务数量（1-10）
/// - **失败回推(分)**：失败任务重新进入待执行队列的等待时间（1-1440 分钟）
/// - **执行超时(分)**：单个任务的最大执行时间（1-1440 分钟）
/// - **自动回推延迟(秒)**：任务状态自动转换的延迟时间（0-3600 秒）
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，用于获取当前设置值
///
/// # 返回值
///
/// 返回构建好的设置模态窗口 UI 元素，类型为 `Element<'_, Message>`
///
/// # 示例
///
/// ```ignore
/// let settings_modal = build_task_settings_modal(&app);
/// // 将 settings_modal 添加到 UI 树中进行渲染
/// ```
pub fn build_task_settings_modal(app: &App) -> Element<'_, Message> {
    fn input_label(label: &str) -> Element<'_, Message> {
        text(label).size(12).style(settings_muted_text_style).into()
    }

    // 定义文本输入框的样式
    //
    // 根据当前主题生成文本输入框的视觉样式
    //
    // # 参数
    //
    // * `theme` - 当前 iced 主题
    // * `_status` - 文本输入框的当前状态（未使用）
    //
    // # 返回值
    //
    // 返回文本输入框的样式配置
    fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
        settings_text_input_style(theme, status)
    }

    fn field_row<'a>(
        label: &'static str,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        row![
            column![text(label).size(13), input_label("数值会立即作用于当前项目任务看板。"),]
                .spacing(4)
                .width(Length::Fixed(168.0)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(20)
        .align_y(Alignment::Center)
        .into()
    }

    let tab_button = |label: &'static str, tab: TaskBoardSettingsModalTab, selected: bool| {
        button(text(label).size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SelectSettingsModalTab(tab)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                let palette = theme.extended_palette();
                let is_dark = theme.palette().background.r
                    + theme.palette().background.g
                    + theme.palette().background.b
                    < 1.5;
                let background = if selected {
                    Some(Background::Color(if is_dark {
                        theme.palette().primary.scale_alpha(0.18)
                    } else {
                        theme.palette().primary.scale_alpha(0.10)
                    }))
                } else if matches!(status, iced::widget::button::Status::Hovered) {
                    Some(Background::Color(if is_dark {
                        palette.background.weak.color.scale_alpha(0.84)
                    } else {
                        Color::WHITE.scale_alpha(0.92)
                    }))
                } else {
                    Some(Background::Color(if is_dark {
                        palette.background.base.color.scale_alpha(0.56)
                    } else {
                        Color::WHITE.scale_alpha(0.78)
                    }))
                };

                iced::widget::button::Style {
                    background,
                    text_color: if selected {
                        theme.palette().primary
                    } else {
                        theme.palette().text
                    },
                    border: Border {
                        radius: 999.0.into(),
                        width: 1.0,
                        color: if selected {
                            theme.palette().primary.scale_alpha(0.45)
                        } else {
                            palette.background.strong.color.scale_alpha(0.62)
                        },
                    },
                    shadow: if selected {
                        iced::Shadow {
                            color: theme.palette().primary.scale_alpha(if is_dark {
                                0.18
                            } else {
                                0.08
                            }),
                            offset: iced::Vector::new(0.0, 8.0),
                            blur_radius: 18.0,
                        }
                    } else {
                        iced::Shadow::default()
                    },
                    ..Default::default()
                }
            })
    };

    // 构建设置模态窗口标题栏
    let header = row![
        text("任务设置")
            .size(18)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        Space::new().width(Length::Fill),
        settings_close_button(Message::TaskBoard(TaskBoardMessage::CloseSettingsModal)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    // 构建并发数设置行
    // 包含减少按钮、输入框和增加按钮，值范围为 1-10
    let max_concurrent = app.task_board_settings.max_concurrent;
    let max_concurrent_text = max_concurrent.to_string();
    let current_refresh_interval_seconds =
        app.task_board_settings.refresh_interval_seconds.clamp(1, 3600);
    let refresh_interval_seconds_text = current_refresh_interval_seconds.to_string();
    let current_scheduler_tick_interval_seconds =
        app.task_board_settings.scheduler_tick_interval_seconds.clamp(1, 60);
    let scheduler_tick_interval_seconds_text = current_scheduler_tick_interval_seconds.to_string();
    let current_auto_promote_tick_interval_seconds =
        app.task_board_settings.auto_promote_tick_interval_seconds.clamp(1, 3600);
    let auto_promote_tick_interval_seconds_text =
        current_auto_promote_tick_interval_seconds.to_string();
    let max_concurrent_row = row![
        input_label("并发数"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(
                max_concurrent.saturating_sub(1).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("数量", &max_concurrent_text)
            .on_input(move |value| {
                // 解析输入值并限制在有效范围内
                let count = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .map(|v| v.clamp(1, 10))
                    .unwrap_or(max_concurrent.clamp(1, 10));
                Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(count))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(
                max_concurrent.saturating_add(1).min(10),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let auto_refresh_toggle_row = row![
        input_label("自动刷新"),
        Space::new().width(Length::Fill),
        toggler(app.task_board_settings.auto_refresh)
            .on_toggle(|enabled| Message::TaskBoard(TaskBoardMessage::ToggleAutoRefresh(enabled))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let dispatch_mode = app.task_board_settings.agent_scheduler_dispatch_mode;
    let dispatch_mode_button = |label: &'static str, mode: AgentSchedulerDispatchModeDto| {
        let active = dispatch_mode == mode;
        button(text(label).size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAgentSchedulerDispatchMode(mode)))
            .padding([4, 10])
            .style(move |theme, status| {
                if active {
                    button_style_primary(theme, status)
                } else {
                    button_style_secondary(theme, status)
                }
            })
    };
    let dispatch_mode_row = row![
        input_label("调度方式"),
        Space::new().width(Length::Fill),
        dispatch_mode_button("本地", AgentSchedulerDispatchModeDto::Local),
        dispatch_mode_button("自动", AgentSchedulerDispatchModeDto::Auto),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let refresh_interval_row = row![
        input_label("刷新间隔(秒)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(
                current_refresh_interval_seconds.saturating_sub(5).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("秒", &refresh_interval_seconds_text)
            .on_input(move |value| {
                let seconds = value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|v| v.clamp(1, 3600))
                    .unwrap_or(current_refresh_interval_seconds);
                Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(seconds))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(
                current_refresh_interval_seconds.saturating_add(5).min(3600),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let scheduler_tick_interval_row = row![
        input_label("执行唤醒间隔(秒)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(
                current_scheduler_tick_interval_seconds.saturating_sub(1).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("秒", &scheduler_tick_interval_seconds_text)
            .on_input(move |value| {
                let seconds = value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|v| v.clamp(1, 60))
                    .unwrap_or(current_scheduler_tick_interval_seconds);
                Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(seconds))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(
                current_scheduler_tick_interval_seconds.saturating_add(1).min(60),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let auto_promote_tick_interval_row = row![
        input_label("任务池回推间隔(秒)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(
                current_auto_promote_tick_interval_seconds.saturating_sub(5).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("秒", &auto_promote_tick_interval_seconds_text)
            .on_input(move |value| {
                let seconds = value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|v| v.clamp(1, 3600))
                    .unwrap_or(current_auto_promote_tick_interval_seconds);
                Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(seconds))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(
                current_auto_promote_tick_interval_seconds.saturating_add(5).min(3600),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 构建失败回推时间设置行
    // 值范围为 1-1440 分钟（最多 24 小时）
    let current_retry_minutes = app.task_board_settings.failed_retry_minutes;
    let retry_minutes_text = current_retry_minutes.to_string();
    let retry_row = row![
        input_label("失败回推(分)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(
                current_retry_minutes.saturating_sub(1).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("分钟", &retry_minutes_text)
            .on_input(move |value| {
                // 解析输入值并限制在有效范围内
                let minutes = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .map(|v| v.clamp(1, 1440))
                    .unwrap_or(current_retry_minutes.clamp(1, 1440));
                Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(minutes))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(
                current_retry_minutes.saturating_add(1).min(1440),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let current_retry_max_attempts = app.task_board_settings.failed_retry_max_attempts;
    let retry_max_attempts_text = current_retry_max_attempts.to_string();
    let retry_max_attempts_row = row![
        input_label("最大重试(次)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(
                current_retry_max_attempts.saturating_sub(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("0-10", &retry_max_attempts_text)
            .on_input(move |value| {
                let attempts = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .map(|v| v.clamp(0, 10))
                    .unwrap_or(current_retry_max_attempts.clamp(0, 10));
                Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(attempts))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(
                current_retry_max_attempts.saturating_add(1).min(10),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 构建执行超时时间设置行
    // 值范围为 1-1440 分钟（最多 24 小时）
    let current_timeout_minutes = app.task_board_settings.running_timeout_minutes;
    let timeout_minutes_text = current_timeout_minutes.to_string();
    let timeout_row = row![
        input_label("执行超时(分)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(
                current_timeout_minutes.saturating_sub(1).max(1),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("分钟", &timeout_minutes_text)
            .on_input(move |value| {
                // 解析输入值并限制在有效范围内
                let minutes = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .map(|v| v.clamp(1, 1440))
                    .unwrap_or(current_timeout_minutes.clamp(1, 1440));
                Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(minutes))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(
                current_timeout_minutes.saturating_add(1).min(1440),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let recycle_on_finish_row = row![
        input_label("完成后回收 worktree"),
        Space::new().width(Length::Fill),
        button(
            text(if app.task_board_settings.recycle_worktree_on_task_finish {
                "开启"
            } else {
                "关闭"
            })
            .size(12)
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::ToggleRecycleWorktreeOnTaskFinish(
            !app.task_board_settings.recycle_worktree_on_task_finish,
        )))
        .padding([4, 10])
        .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let current_pr_stall_timeout_seconds =
        app.task_board_settings.pr_submitted_stall_timeout_seconds;
    let pr_stall_timeout_seconds_text = current_pr_stall_timeout_seconds.to_string();
    let pr_stall_timeout_row = row![
        input_label("合并锁超时(秒)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(
                current_pr_stall_timeout_seconds.saturating_sub(5).max(5),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("秒", &pr_stall_timeout_seconds_text)
            .on_input(move |value| {
                let seconds = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .map(|v| v.clamp(5, 3600))
                    .unwrap_or(current_pr_stall_timeout_seconds.clamp(5, 3600));
                Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(seconds))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(
                current_pr_stall_timeout_seconds.saturating_add(5).min(3600),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 构建自动回推延迟设置行
    // 值范围为 0-3600 秒（最多 1 小时）
    let current_delay_seconds = app.task_board_settings.auto_promote_delay_seconds;
    let delay_seconds_text = current_delay_seconds.to_string();
    let delay_row = row![
        input_label("自动回推延迟(秒)"),
        Space::new().width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(
                current_delay_seconds.saturating_sub(5),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
        text_input("秒", &delay_seconds_text)
            .on_input(move |value| {
                // 解析输入值并限制在有效范围内
                let seconds = value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|v| v.clamp(0, 3600))
                    .unwrap_or(current_delay_seconds.clamp(0, 3600));
                Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(seconds))
            })
            .padding([4, 8])
            .size(12)
            .width(Length::Fixed(52.0))
            .style(input_style),
        button(text("+").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(
                current_delay_seconds.saturating_add(5).min(3600),
            )))
            .padding([4, 8])
            .style(button_style_secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let refresh_tab = column![
        settings_section_card(
            "刷新与自动化",
            "统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。",
        ),
        settings_panel(
            column![
                field_row("自动刷新", auto_refresh_toggle_row),
                field_row("调度方式", dispatch_mode_row),
                field_row("刷新间隔", refresh_interval_row),
                field_row("执行唤醒间隔", scheduler_tick_interval_row),
                field_row("任务池回推间隔", auto_promote_tick_interval_row),
                field_row("自动回推延迟", delay_row),
            ]
            .spacing(12)
        ),
    ]
    .spacing(12);

    let scheduling_tab = column![
        settings_section_card("并发与保护", "控制任务吞吐、失败重试、超时与 worktree 回收策略。",),
        settings_panel(
            column![
                field_row("并发数", max_concurrent_row),
                field_row("失败回推", retry_row),
                field_row("最大重试", retry_max_attempts_row),
                field_row("执行超时", timeout_row),
                field_row("完成后回收", recycle_on_finish_row),
                field_row("合并锁超时", pr_stall_timeout_row),
            ]
            .spacing(12)
        ),
    ]
    .spacing(12);

    let active_tab_content: Element<'_, Message> = match app.task_board_settings_modal_tab {
        TaskBoardSettingsModalTab::Refresh => refresh_tab.into(),
        TaskBoardSettingsModalTab::Scheduling => scheduling_tab.into(),
    };

    let active_tab_hint = match app.task_board_settings_modal_tab {
        TaskBoardSettingsModalTab::Refresh => "自动刷新、执行唤醒与任务池回推参数",
        TaskBoardSettingsModalTab::Scheduling => "并发、失败重试、超时与 worktree 回收策略",
    };

    let tabs = row![
        tab_button(
            "刷新与自动化",
            TaskBoardSettingsModalTab::Refresh,
            app.task_board_settings_modal_tab == TaskBoardSettingsModalTab::Refresh,
        ),
        tab_button(
            "并发与保护",
            TaskBoardSettingsModalTab::Scheduling,
            app.task_board_settings_modal_tab == TaskBoardSettingsModalTab::Scheduling,
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let settings_content = column![
        header,
        tabs,
        text(active_tab_hint).size(11).style(settings_muted_text_style),
        scrollable(container(active_tab_content).width(Length::Fixed(492.0)))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .width(Length::Fill)
            .height(Length::Fixed(340.0)),
    ]
    .spacing(14)
    .width(Length::Fixed(520.0));

    // 返回带有容器样式和阴影效果的完整设置模态窗口
    settings_modal_card(settings_content).width(Length::Fixed(580.0)).into()
}

/// 构建任务上下文菜单
///
/// 该函数创建一个任务项的右键上下文菜单，提供常用的任务操作选项。
/// 菜单以弹出层形式显示，包含复制、删除和归档等操作
///
/// # 菜单选项
///
/// - **复制任务**：创建当前任务的副本
/// - **删除任务**：永久删除该任务
/// - **归档任务**：将任务移动到归档区
///
/// # 参数
///
/// * `task` - 要操作的任务对象，包含任务 ID 等必要信息
///
/// # 返回值
///
/// 返回构建好的上下文菜单 UI 元素，类型为 `Element<'static, Message>`
/// 使用 `'static` 生命周期是因为菜单内容不依赖于外部引用
///
/// # 示例
///
/// ```ignore
/// let context_menu = build_context_menu(task);
/// // 在右键点击事件中显示 context_menu
/// ```
pub fn build_context_menu(task: Task) -> Element<'static, Message> {
    let retry_btn = (task.status == crate::app::task::TaskStatus::Failed).then(|| {
        button(
            row![text("↻").size(12), Space::new().width(8.0), text("手工重试").size(12),]
                .align_y(Alignment::Center),
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::ExecuteTask { task_id: task.id.clone() }))
        .padding([6, 10])
        .width(Length::Fill)
        .style(button_style_secondary)
    });

    // 复制任务按钮：点击触发复制任务的消息
    let copy_btn = button(
        row![text("📄").size(12), Space::new().width(8.0), text("复制任务").size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::DuplicateTask(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 删除任务按钮：点击触发删除任务的消息
    let delete_btn = button(
        row![text("🗑️").size(12), Space::new().width(8.0), text("删除任务").size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::TaskDeleted(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 归档任务按钮：点击触发归档任务的消息
    let archive_btn = button(
        row![text("📦").size(12), Space::new().width(8.0), text("归档任务").size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::TaskArchived(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 组装菜单项
    let mut menu = column![].spacing(2).width(Length::Fixed(132.0));
    if let Some(retry_btn) = retry_btn {
        menu = menu.push(retry_btn);
    }
    menu = menu.push(copy_btn).push(delete_btn).push(archive_btn);

    // 返回带有容器样式和阴影效果的完整菜单
    container(menu)
        .padding(6)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(theme.palette().background)),
            border: Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.18),
                offset: iced::Vector::new(1.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
#[path = "modals_tests.rs"]
mod modals_tests;
