//! # 子任务编辑器模块
//!
//! 本模块提供任务看板中子任务的编辑和显示功能。
//!
//! ## 主要功能
//!
//! - **编辑模式子任务构建**：构建已存在任务的子任务编辑界面
//! - **草稿模式子任务构建**：构建新任务草稿的子任务编辑界面
//! - **子任务操作按钮**：上移、下移、删除子任务的控制按钮
//! - **任务日志显示**：展示任务的执行日志，支持文件路径和URL的交互
//!
//! ## 模块结构
//!
//! - 公开函数：`build_edit_mode_subtasks`、`build_draft_mode_subtasks`、`build_task_logs`
//! - 私有辅助函数：按钮构建、URL解析、路径转换等
//!
//! ## 使用场景
//!
//! 该模块主要用于任务看板面板中，当用户查看或编辑任务详情时，
//! 提供子任务列表的展示和交互功能。

use crate::app::components::text_editor_context_menu::{
    TextEditorContextMenuMessages, TextEditorContextMenuState, wrap_with_context_menu,
};
use crate::app::components::text_editor_scroll_panel::{
    TextEditorScrollPanelMetrics, text_editor_scroll_panel,
};
use crate::app::message::TaskBoardMessage;
use crate::app::task::Task;
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, responsive, row, scrollable, text, text_editor, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Size, Theme};

use super::super::common::{
    SUBTASK_BADGE_SIZE, button_style_danger, button_style_primary, button_style_secondary,
};
use super::styles::{
    disabled_arrow_button_style, input_style, subtask_badge_style, subtask_card_style,
    subtask_status_badge_style, subtask_status_icon, subtask_status_label,
    subtask_status_pill_style, subtask_status_text_style,
};

const SUBTASK_LIST_MAX_HEIGHT: f32 = 260.0;
const SUBTASK_SCROLLBAR_WIDTH: f32 = 4.0;
const SUBTASK_SCROLLBAR_GUTTER: f32 = 12.0;

fn format_subtask_started_at(ms: u64) -> String {
    static FMT: once_cell::sync::Lazy<Vec<time::format_description::FormatItem<'static>>> =
        once_cell::sync::Lazy::new(|| {
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                .unwrap_or_default()
        });

    let nanos = (ms as i128).saturating_mul(1_000_000);
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| dt.format(&FMT).ok())
        .unwrap_or_else(|| "-".to_string())
}

/// 构建编辑模式下的子任务列表
///
/// 该函数用于构建已存在任务的子任务编辑界面，包含：
/// - 子任务序号徽章（可点击切换完成状态）
/// - 子任务内容输入框
/// - 上移/下移/删除操作按钮
///
/// # 参数
///
/// - `app`: 应用程序状态引用，用于获取全局状态
/// - `task`: 当前正在编辑的任务引用
///
/// # 返回值
///
/// 返回一个元组：
/// - `Element<'a, Message>`: 子任务列表容器
/// - `Element<'a, Message>`: 添加新子任务的区域
///
/// # 示例
///
/// ```ignore
/// let (subtasks_view, add_section) = build_edit_mode_subtasks(&app, &task);
/// // 将两个元素分别渲染到界面上
/// ```
pub fn build_edit_mode_subtasks<'a>(
    app: &'a App,
    task: &'a Task,
) -> (Element<'a, Message>, Element<'a, Message>) {
    // 初始化子任务列容器，设置子元素间距为6像素
    let mut subtasks_col = column![].spacing(6);
    let now_ms = crate::app::time::now_ms();

    // 遍历任务的所有子任务，为每个子任务构建UI组件
    for (idx, subtask) in task.subtasks.iter().enumerate() {
        let status = subtask.status;
        let badge_label = subtask_status_icon(status, now_ms);

        // 克隆任务ID和子任务ID，用于闭包捕获
        let task_id_for_toggle = task.id.clone();
        let subtask_id_for_toggle = subtask.id.clone();

        // 构建子任务序号徽章按钮
        // 点击徽章可切换子任务的完成状态
        let index_badge = button(
            container(
                text(badge_label)
                    .size(10)
                    .style(move |theme: &Theme| subtask_status_text_style(theme, status)),
            )
            .width(Length::Fixed(SUBTASK_BADGE_SIZE))
            .height(Length::Fixed(SUBTASK_BADGE_SIZE))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::ToggleSubTaskCompleted {
            task_id: task_id_for_toggle,
            subtask_id: subtask_id_for_toggle,
        }))
        .padding(0)
        .width(Length::Fixed(SUBTASK_BADGE_SIZE))
        .height(Length::Fixed(SUBTASK_BADGE_SIZE))
        .style(move |theme, button_status| {
            subtask_status_badge_style(theme, button_status, status)
        });

        // 克隆任务ID和子任务ID，用于输入框闭包捕获
        let task_id_for_update = task.id.clone();
        let subtask_id_for_update = subtask.id.clone();

        // 构建子任务内容输入框
        // 输入内容变更时触发更新消息
        let subtask_input = text_input("输入子任务内容...", &subtask.content)
            .on_input(move |v| {
                Message::TaskBoard(TaskBoardMessage::UpdateSubTaskContent {
                    task_id: task_id_for_update.clone(),
                    subtask_id: subtask_id_for_update.clone(),
                    content: v,
                })
            })
            .padding([6, 8])
            .size(12)
            .width(Length::Fill)
            .style(input_style);

        // 初始化子任务行，包含徽章和输入框
        let mut subtask_row =
            row![index_badge, subtask_input].spacing(6).align_y(Alignment::Center);

        // 添加上移按钮
        let up_btn = build_up_button(idx, task, true);
        subtask_row = subtask_row.push(up_btn);

        // 添加下移按钮
        let down_btn = build_down_button(idx, task.subtasks.len(), task, true);
        subtask_row = subtask_row.push(down_btn);

        // 添加删除按钮
        let delete_btn = build_delete_button(task, subtask.id.as_str(), true);
        subtask_row = subtask_row.push(delete_btn);

        let timing_label = subtask
            .display_execution_duration_ms(now_ms)
            .map(|ms| format!("耗时 {}s", ms / 1000))
            .unwrap_or_else(|| "耗时 -".to_string());
        let started_label = subtask
            .execution_started_at_ms
            .map(|ms| format!("开始 {}", format_subtask_started_at(ms)))
            .unwrap_or_else(|| "开始 -".to_string());
        let status_pill = container(
            text(subtask_status_label(status))
                .size(10)
                .style(move |theme: &Theme| subtask_status_text_style(theme, status)),
        )
        .padding([2, 6])
        .style(move |theme: &Theme| subtask_status_pill_style(theme, status));
        let meta_row =
            row![status_pill, text(started_label).size(10), text(timing_label).size(10),]
                .spacing(8)
                .align_y(Alignment::Center);

        // 将子任务行包装在卡片容器中，添加到子任务列
        let subtask_card = container(column![subtask_row, meta_row].spacing(4))
            .padding([6, 10])
            .width(Length::Fill)
            .style(subtask_card_style);
        subtasks_col = subtasks_col.push(subtask_card);
    }

    // 构建添加新子任务的输入区域
    let add_section = build_add_subtask_section(app, &task.id);

    let subtasks_view = container(
        scrollable(container(subtasks_col).width(Length::Fill).padding(iced::Padding {
            top: 0.0,
            right: SUBTASK_SCROLLBAR_GUTTER,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(Direction::Vertical(
            Scrollbar::new().width(SUBTASK_SCROLLBAR_WIDTH).scroller_width(SUBTASK_SCROLLBAR_WIDTH),
        ))
        .height(Length::Shrink)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .max_height(SUBTASK_LIST_MAX_HEIGHT);

    (subtasks_view.into(), add_section)
}

/// 构建草稿模式下的子任务列表
///
/// 该函数用于构建新任务草稿的子任务编辑界面。
/// 草稿模式下的子任务尚未持久化，仅存在于内存中。
///
/// # 参数
///
/// - `app`: 应用程序状态引用，从中获取草稿数据
///
/// # 返回值
///
/// 返回包含完整子任务编辑界面的UI元素，包括：
/// - 标题 "子任务"
/// - 所有草稿子任务的编辑行
/// - "新增子任务" 按钮
///
/// # 示例
///
/// ```ignore
/// let subtasks_view = build_draft_mode_subtasks(&app);
/// // 将视图渲染到界面上
/// ```
pub fn build_draft_mode_subtasks<'a>(app: &'a App) -> Element<'a, Message> {
    // 初始化子任务列容器
    let mut subtasks_col = column![].spacing(6);

    // 遍历草稿中的所有子任务
    for (idx, subtask) in app.task_board_draft.subtasks.iter().enumerate() {
        // 构建序号徽章（草稿模式下不显示完成状态）
        let index_badge = button(
            container(text(format!("{}", idx + 1)).size(10).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.base.text),
                }
            }))
            .width(Length::Fixed(SUBTASK_BADGE_SIZE))
            .height(Length::Fixed(SUBTASK_BADGE_SIZE))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .padding(0)
        .width(Length::Fixed(SUBTASK_BADGE_SIZE))
        .height(Length::Fixed(SUBTASK_BADGE_SIZE))
        .style(subtask_badge_style);

        // 构建子任务内容输入框
        // 草稿模式下使用索引而非ID来标识子任务
        let subtask_input = text_input("输入子任务内容...", subtask)
            .on_input(move |v| {
                Message::TaskBoard(TaskBoardMessage::UpdateDraftSubtask { index: idx, value: v })
            })
            .padding([6, 8])
            .size(12)
            .width(Length::Fill)
            .style(input_style);

        // 构建操作按钮（草稿版本）
        let up_btn = build_draft_up_button(idx);
        let down_btn = build_draft_down_button(idx, app.task_board_draft.subtasks.len());
        let delete_btn = build_draft_delete_button(idx);

        // 组装子任务行
        let subtask_row = row![index_badge, subtask_input, up_btn, down_btn, delete_btn]
            .spacing(6)
            .align_y(Alignment::Center);

        // 包装为卡片容器并添加到列中
        let subtask_card =
            container(subtask_row).padding([6, 10]).width(Length::Fill).style(subtask_card_style);
        subtasks_col = subtasks_col.push(subtask_card);
    }

    // 构建"新增子任务"按钮
    let add_subtask_btn = button(text("新增子任务").size(11))
        .on_press(Message::TaskBoard(TaskBoardMessage::AddDraftSubtask))
        .padding([6, 10])
        .style(button_style_secondary);

    // 构建标题文本
    let subtasks_title = text("子任务")
        .size(13)
        .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() });

    // 组装完整的草稿子任务视图
    column![
        subtasks_title,
        Space::new().height(6.0),
        subtasks_col,
        Space::new().height(6.0),
        row![add_subtask_btn, Space::new().width(Length::Fill)].align_y(Alignment::Center),
    ]
    .into()
}

/// 构建添加子任务的输入区域
///
/// 该函数创建一个包含输入框和添加按钮的区域，
/// 用于向已存在的任务添加新的子任务。
///
/// # 参数
///
/// - `app`: 应用程序状态引用，从中获取新子任务内容
/// - `task_id`: 目标任务ID
///
/// # 返回值
///
/// 返回包含以下组件的UI元素：
/// - "添加子任务" 标题
/// - 子任务内容输入框
/// - "添加" 按钮
///
/// # 示例
///
/// ```ignore
/// let add_section = build_add_subtask_section(&app, "task-123");
/// ```
fn build_add_subtask_section<'a>(app: &'a App, task_id: &str) -> Element<'a, Message> {
    // 标题文本
    let add_subtask_title = text("添加子任务").size(12);

    // 子任务内容输入框
    // 绑定到应用状态中的 task_board_new_subtask_content 字段
    let add_subtask_input = text_input("输入子任务内容...", &app.task_board_new_subtask_content)
        .on_input(|v| Message::TaskBoard(TaskBoardMessage::UpdateNewSubtaskContent(v)))
        .padding([6, 8])
        .size(12)
        .width(Length::Fill)
        .style(input_style);

    // 克隆数据用于闭包捕获
    let content_for_add = app.task_board_new_subtask_content.clone();
    let task_id_for_add = task_id.to_string();

    // 添加按钮
    // 点击时发送添加子任务消息
    let add_subtask_btn = button(text("添加").size(11))
        .on_press(Message::TaskBoard(TaskBoardMessage::AddSubTask {
            task_id: task_id_for_add,
            content: content_for_add,
        }))
        .padding([6, 12])
        .style(button_style_primary);

    // 组装添加区域布局
    column![
        add_subtask_title,
        Space::new().height(4.0),
        row![add_subtask_input, add_subtask_btn].spacing(8).width(Length::Fill),
    ]
    .into()
}

/// 构建上移按钮
///
/// 该函数创建用于将子任务在列表中向上移动的按钮。
/// 如果子任务已经在最顶部，则按钮处于禁用状态。
///
/// # 参数
///
/// - `idx`: 当前子任务的索引位置
/// - `task`: 任务引用，用于获取任务ID和子任务ID
/// - `is_edit_mode`: 是否为编辑模式（true为编辑模式，false为草稿模式）
///
/// # 返回值
///
/// 返回上移按钮的UI元素：
/// - 如果 `idx > 0`：返回可点击的按钮
/// - 如果 `idx == 0`：返回禁用样式的按钮
///
/// # 示例
///
/// ```ignore
/// let up_btn = build_up_button(2, &task, true);
/// ```
fn build_up_button<'a>(idx: usize, task: &Task, is_edit_mode: bool) -> Element<'a, Message> {
    // 只有不在顶部时才可点击
    if idx > 0 {
        button(text("↑").size(10))
            .on_press(Message::TaskBoard(if is_edit_mode {
                // 编辑模式：使用任务ID和子任务ID
                TaskBoardMessage::MoveSubTaskUp {
                    task_id: task.id.clone(),
                    subtask_id: task.subtasks[idx].id.clone(),
                }
            } else {
                // 草稿模式：使用索引
                TaskBoardMessage::MoveDraftSubtaskUp(idx)
            }))
            .padding([2, 6])
            .style(button_style_secondary)
            .into()
    } else {
        // 已在顶部，显示禁用样式
        button(text("↑").size(10))
            .padding([2, 6])
            .style(|theme: &Theme, _| disabled_arrow_button_style(theme))
            .into()
    }
}

/// 构建下移按钮
///
/// 该函数创建用于将子任务在列表中向下移动的按钮。
/// 如果子任务已经在最底部，则按钮处于禁用状态。
///
/// # 参数
///
/// - `idx`: 当前子任务的索引位置
/// - `total`: 子任务总数
/// - `task`: 任务引用，用于获取任务ID和子任务ID
/// - `is_edit_mode`: 是否为编辑模式
///
/// # 返回值
///
/// 返回下移按钮的UI元素：
/// - 如果 `idx < total - 1`：返回可点击的按钮
/// - 如果 `idx == total - 1`：返回禁用样式的按钮
///
/// # 示例
///
/// ```ignore
/// let down_btn = build_down_button(1, 5, &task, true);
/// ```
fn build_down_button<'a>(
    idx: usize,
    total: usize,
    task: &Task,
    is_edit_mode: bool,
) -> Element<'a, Message> {
    // 只有不在底部时才可点击
    if idx < total - 1 {
        button(text("↓").size(10))
            .on_press(Message::TaskBoard(if is_edit_mode {
                // 编辑模式：使用任务ID和子任务ID
                TaskBoardMessage::MoveSubTaskDown {
                    task_id: task.id.clone(),
                    subtask_id: task.subtasks[idx].id.clone(),
                }
            } else {
                // 草稿模式：使用索引
                TaskBoardMessage::MoveDraftSubtaskDown(idx)
            }))
            .padding([2, 6])
            .style(button_style_secondary)
            .into()
    } else {
        // 已在底部，显示禁用样式
        button(text("↓").size(10))
            .padding([2, 6])
            .style(|theme: &Theme, _| disabled_arrow_button_style(theme))
            .into()
    }
}

/// 构建删除按钮
///
/// 该函数创建用于删除子任务的按钮。
///
/// # 参数
///
/// - `task`: 任务引用，用于获取任务ID
/// - `subtask_id`: 要删除的子任务ID
/// - `_is_edit_mode`: 编辑模式标志（当前未使用，保留用于扩展）
///
/// # 返回值
///
/// 返回删除按钮的UI元素，使用危险样式（通常为红色）
///
/// # 示例
///
/// ```ignore
/// let delete_btn = build_delete_button(&task, "subtask-456", true);
/// ```
fn build_delete_button<'a>(
    task: &Task,
    subtask_id: &str,
    _is_edit_mode: bool,
) -> Element<'a, Message> {
    button(text("×").size(10))
        .on_press(Message::TaskBoard(TaskBoardMessage::RemoveSubTask {
            task_id: task.id.clone(),
            subtask_id: subtask_id.to_string(),
        }))
        .padding([2, 6])
        .style(button_style_danger)
        .into()
}

/// 构建草稿模式的上移按钮
///
/// 该函数是 `build_up_button` 的草稿模式专用版本，
/// 使用索引而非ID来标识子任务。
///
/// # 参数
///
/// - `idx`: 当前子任务的索引位置
///
/// # 返回值
///
/// 返回上移按钮的UI元素：
/// - 如果 `idx > 0`：返回可点击的按钮
/// - 如果 `idx == 0`：返回禁用样式的按钮
///
/// # 示例
///
/// ```ignore
/// let up_btn = build_draft_up_button(2);
/// ```
fn build_draft_up_button<'a>(idx: usize) -> Element<'a, Message> {
    // 只有不在顶部时才可点击
    if idx > 0 {
        button(text("↑").size(10))
            .on_press(Message::TaskBoard(TaskBoardMessage::MoveDraftSubtaskUp(idx)))
            .padding([2, 6])
            .style(button_style_secondary)
            .into()
    } else {
        // 已在顶部，显示禁用样式
        button(text("↑").size(10))
            .padding([2, 6])
            .style(|theme: &Theme, _| disabled_arrow_button_style(theme))
            .into()
    }
}

/// 构建草稿模式的下移按钮
///
/// 该函数是 `build_down_button` 的草稿模式专用版本，
/// 使用索引而非ID来标识子任务。
///
/// # 参数
///
/// - `idx`: 当前子任务的索引位置
/// - `total`: 子任务总数
///
/// # 返回值
///
/// 返回下移按钮的UI元素：
/// - 如果 `idx + 1 < total`：返回可点击的按钮
/// - 否则：返回禁用样式的按钮
///
/// # 示例
///
/// ```ignore
/// let down_btn = build_draft_down_button(1, 5);
/// ```
fn build_draft_down_button<'a>(idx: usize, total: usize) -> Element<'a, Message> {
    // 只有不在底部时才可点击
    if idx + 1 < total {
        button(text("↓").size(10))
            .on_press(Message::TaskBoard(TaskBoardMessage::MoveDraftSubtaskDown(idx)))
            .padding([2, 6])
            .style(button_style_secondary)
            .into()
    } else {
        // 已在底部，显示禁用样式
        button(text("↓").size(10))
            .padding([2, 6])
            .style(|theme: &Theme, _| disabled_arrow_button_style(theme))
            .into()
    }
}

/// 构建草稿模式的删除按钮
///
/// 该函数是 `build_delete_button` 的草稿模式专用版本，
/// 使用索引而非ID来标识子任务。
///
/// # 参数
///
/// - `idx`: 要删除的子任务索引
///
/// # 返回值
///
/// 返回删除按钮的UI元素，使用危险样式
///
/// # 示例
///
/// ```ignore
/// let delete_btn = build_draft_delete_button(3);
/// ```
fn build_draft_delete_button<'a>(idx: usize) -> Element<'a, Message> {
    button(text("×").size(10))
        .on_press(Message::TaskBoard(TaskBoardMessage::RemoveDraftSubtask(idx)))
        .padding([2, 6])
        .style(button_style_danger)
        .into()
}

/// 构建任务日志显示视图
///
/// 该函数创建一个可滚动的日志显示区域，展示任务的执行日志。
/// 支持以下功能：
/// - 自动限制显示的日志条数（最多400条）
/// - 时间戳格式化（[HH:MM:SS]）
/// - 文件路径识别和可点击链接
/// - URL识别和可点击链接
///
/// # 参数
///
/// - `task`: 任务引用，从中获取日志数据
///
/// # 返回值
///
/// 返回包含日志查看区域的UI元素，填满调用方提供的可用高度
///
/// # 特殊处理
///
/// 1. **选择复制**：基于只读文本编辑器，支持鼠标框选和系统复制快捷键
/// 2. **防止误编辑**：编辑动作在消息层被忽略，仅保留滚动和文本选择能力
/// 3. **日志限制**：只显示最新的400条日志，避免性能问题
///
/// # 示例
///
/// ```ignore
/// let logs_view = build_task_logs(&app);
/// ```
pub fn build_task_logs<'a>(app: &'a App) -> Element<'a, Message> {
    let panel = responsive(move |size| build_task_logs_panel(app, size)).height(Length::Fill);

    container(panel).width(Length::Fill).height(Length::Fill).into()
}

fn build_task_logs_panel<'a>(app: &'a App, size: Size) -> Element<'a, Message> {
    let editor = text_editor(&app.task_board_logs_editor)
        .id(app.task_board_logs_editor_id.clone())
        .on_action(|action| Message::TaskBoard(TaskBoardMessage::LogsViewerEditorAction(action)))
        .font(iced::Font::with_name("JetBrains Mono"))
        .size(11)
        .padding(0)
        .height(Length::Shrink)
        .style(|theme: &Theme, _status| {
            let p = theme.extended_palette();
            text_editor::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                value: theme.palette().text,
                selection: theme.palette().primary.scale_alpha(0.30),
                placeholder: p.secondary.strong.color.scale_alpha(0.55),
            }
        });

    let editor = wrap_with_context_menu(
        editor,
        TextEditorContextMenuState {
            open: app.task_board_logs_context_menu_open,
            position: app.task_board_logs_context_menu_pos,
        },
        |point| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerOpenContextMenu {
                x: point.x,
                y: point.y,
            })
        },
        TextEditorContextMenuMessages {
            close: Message::TaskBoard(TaskBoardMessage::LogsViewerCloseContextMenu),
            copy: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuCopy),
            cut: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuCut),
            paste: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuPaste),
            delete: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuDelete),
        },
    );

    text_editor_scroll_panel(
        editor,
        size,
        TextEditorScrollPanelMetrics {
            viewport_padding: 24.0,
            line_height: app.current_line_height,
            line_count: app.task_board_logs_editor.line_count(),
            scroll_top_line: app.task_board_logs_scroll_top_line,
        },
        |delta, viewport_height| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerEditorWheelScrolled {
                delta,
                viewport_height,
            })
        },
        |top_line, viewport_height| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerScrollbarChanged {
                top_line,
                viewport_height,
            })
        },
    )
}

#[cfg(test)]
#[path = "subtask_editor_tests.rs"]
mod subtask_editor_tests;
