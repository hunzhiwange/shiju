//! # 输入面板组件模块
//!
//! 本模块负责构建应用程序的输入面板UI，这是用户与代理进行交互的主要界面。
//! 输入面板包含多个子组件，协同工作以提供完整的输入体验。
//!
//! ## 主要功能
//!
//!
//! - **文本输入编辑器**：用户输入消息的主要区域
//! - **文件附件处理**：支持拖拽添加文件附件
//! - **模型选择**：切换和配置使用的AI模型
//! - **任务模式**：支持高级任务执行模式配置
//! - **队列管理**：显示和管理待处理的消息队列
//! - **使用量显示**：显示当前资源使用情况
//! - **TODO面板**：显示当前活跃的待办事项
//!
//! ## 模块结构
//!
//! - `attachment`: 文件附件相关组件
//! - `drop_area`: 拖拽区域处理
//! - `file_references`: 文件引用提取和渲染
//! - `file_search`: 文件搜索功能
//! - `icons`: 图标组件（公开模块）
//! - `input_editor`: 输入编辑器组件
//! - `model_popover`: 模型选择弹出框
//! - `queue_panel`: 队列面板显示
//! - `send_controls`: 发送控制按钮组
//! - `styles`: 样式定义
//! - `task_mode`: 任务模式表单
//! - `todo_panel`: TODO面板
//! - `usage`: 使用量统计
//! - `usage_button`: 使用量显示按钮

// 子模块声明
pub(crate) mod agent_sessions;
pub(crate) mod attachment;
pub(crate) mod drop_area;
mod file_references;
mod file_search;
pub mod icons;
mod input_editor;
mod model_popover;
mod queue_panel;
mod quick_messages;
pub(crate) mod reasoning_selector;
mod send_controls;
mod slash_palette;
pub(crate) mod styles;
mod task_mode;
pub(crate) mod todo_panel;
pub(crate) mod usage;
mod usage_button;

#[cfg(test)]
#[path = "agent_sessions_tests.rs"]
mod agent_sessions_tests;
#[cfg(test)]
#[path = "drop_area_tests.rs"]
mod drop_area_tests;
#[cfg(test)]
#[path = "file_references_tests.rs"]
mod file_references_tests;
#[cfg(test)]
#[path = "file_search_tests.rs"]
mod file_search_tests;
#[cfg(test)]
#[path = "icons_tests.rs"]
mod icons_tests;
#[cfg(test)]
#[path = "input_editor_tests.rs"]
mod input_editor_tests;
#[cfg(test)]
#[path = "model_popover_tests.rs"]
mod model_popover_tests;
#[cfg(test)]
#[path = "queue_panel_tests.rs"]
mod queue_panel_tests;
#[cfg(test)]
#[path = "reasoning_selector_tests.rs"]
mod reasoning_selector_tests;
#[cfg(test)]
#[path = "send_controls_tests.rs"]
mod send_controls_tests;
#[cfg(test)]
#[path = "slash_palette_tests.rs"]
mod slash_palette_tests;
#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
#[cfg(test)]
#[path = "task_mode_tests.rs"]
mod task_mode_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "todo_panel_tests.rs"]
mod todo_panel_tests;
#[cfg(test)]
#[path = "usage_button_tests.rs"]
mod usage_button_tests;
#[cfg(test)]
#[path = "usage_tests.rs"]
mod usage_tests;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::{
    conversation_reading_column, tool_selector::session_tool_selector_popover,
};
use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::overlays::AboveOverlay;
use crate::app::state::tool_display_name;
use crate::app::theme_catalog::with_panel_background;
use crate::app::{App, Message, TodoPanelPlacement, message};
use drop_area::DropArea;
use file_references::{extract_file_mentions, render_file_references};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};
use icons::icon_svg;
use reasoning_selector::reasoning_effort_selector;
use send_controls::{
    bottom_bar, cancel_button, full_access_button, pool_button, send_button, subagent_mode_button,
    workflow_mode_button,
};
use task_mode::task_mode_form;

pub(super) fn popover_item_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    styles::selectable_list_button_style(theme, status, selected)
}

const ACP_SELECTOR_MAX_HEIGHT: f32 = 240.0;
const ACP_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
const ACP_SELECTOR_LIST_RIGHT_PADDING: f32 = 5.0;

pub(super) fn bottom_bar_round_toggle<'a>(
    icon: Element<'a, Message>,
    tooltip_label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    let button_content = container(icon)
        .width(Length::Fixed(styles::BOTTOM_BAR_ICON_BUTTON_SIZE))
        .height(Length::Fixed(styles::BOTTOM_BAR_ICON_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let toggle = button(button_content)
        .padding(0)
        .style(|theme: &Theme, status| styles::round_icon_button_style(theme, status, true))
        .on_press(on_press);

    Tooltip::new(toggle, dark_tooltip_content(tooltip_label), TooltipPosition::Top).gap(6.0).into()
}

fn session_tool_tab_icon(tab: crate::app::state::SessionToolSelectorTab) -> Icon {
    // 与 slash 面板语义对齐：技能=星标、工具=终端；代理用六边形身份标记，避免像聊天气泡。
    match tab {
        crate::app::state::SessionToolSelectorTab::Agent => Icon::Hexagon,
        crate::app::state::SessionToolSelectorTab::Tools => Icon::Terminal,
        crate::app::state::SessionToolSelectorTab::Skills => Icon::Star,
    }
}

pub(super) fn session_tool_tab_button<'a>(
    app: &'a App,
    tab: crate::app::state::SessionToolSelectorTab,
) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    let active_tab = runtime.tool_selector.active_tab();
    let popover_open = app.show_session_tool_selector_popover;
    let highlight = (popover_open && active_tab == tab)
        || match tab {
            crate::app::state::SessionToolSelectorTab::Agent => runtime.agent.is_some(),
            crate::app::state::SessionToolSelectorTab::Tools => {
                !runtime.tool_selector.selected_manual_tools().is_empty()
            }
            crate::app::state::SessionToolSelectorTab::Skills => {
                !runtime.tool_selector.selected_manual_skills().is_empty()
                    || !runtime.tool_selector.selected_shiju_skills().is_empty()
            }
        };

    let icon: Element<'_, Message> =
        icon_svg(session_tool_tab_icon(tab), styles::BOTTOM_BAR_ICON_SIZE)
            .style(move |theme: &Theme, _| iced::widget::svg::Style {
                color: Some(styles::selector_text_color(theme, highlight)),
            })
            .into();
    let toggle = bottom_bar_round_toggle(
        icon,
        tab.label(),
        Message::Chat(message::ChatMessage::OpenSessionToolSelectorTab(tab)),
    );

    if popover_open && active_tab == tab {
        AboveOverlay::new(toggle, session_tool_selector_popover(app))
            .show(true)
            .gap(6.0)
            .on_close(Message::View(message::ViewMessage::ClosePopovers))
            .into()
    } else {
        toggle
    }
}

pub(super) fn session_tool_tab_buttons<'a>(
    app: &'a App,
) -> (Element<'a, Message>, Element<'a, Message>, Element<'a, Message>) {
    (
        session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Agent),
        session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Tools),
        session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Skills),
    )
}

pub(super) fn acp_selector_button<'a>(
    app: &'a App,
    selected_acp_agent: Option<String>,
    locked: bool,
) -> Option<Element<'a, Message>> {
    let label = selected_acp_agent.as_deref().unwrap_or("ACP 智能体").to_string();
    let icon = acp_agent_icon(&label, styles::BOTTOM_BAR_ICON_SIZE);
    if locked {
        return Some(icon.into());
    }

    let toggle = bottom_bar_round_toggle(
        icon,
        acp_selector_tooltip(&label),
        Message::View(message::ViewMessage::ToggleAcpPopover),
    );

    let selected_for_default = selected_acp_agent.is_none();
    let default_check: Element<'_, Message> = if selected_for_default {
        icon_svg(Icon::Check, 14.0).into()
    } else {
        Space::new().width(Length::Fixed(14.0)).into()
    };

    let default_button = button(
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
    .style(move |theme: &Theme, status| popover_item_style(theme, status, selected_for_default))
    .on_press(Message::Chat(message::ChatMessage::AcpAgentSelected(None)));

    let mut list = column![default_button].spacing(4);
    for agent in app.acp_agents.iter().cloned() {
        let selected = selected_acp_agent.as_ref() == Some(&agent);
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
        .style(move |theme: &Theme, status| popover_item_style(theme, status, selected))
        .on_press(Message::Chat(message::ChatMessage::AcpAgentSelected(Some(agent_for_press))));
        list = list.push(select_btn);
    }

    if app.acp_agents.is_empty() {
        list = list.push(
            container(text("正在加载 ACP 列表，稍后会自动显示。").size(11).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.62)),
                },
            ))
            .padding([4, 2]),
        );
    }

    let popover: Element<'_, Message> = container(
        scrollable(container(list).padding(iced::Padding {
            top: 0.0,
            right: ACP_SELECTOR_LIST_RIGHT_PADDING,
            bottom: 0.0,
            left: 0.0,
        }))
        .id(iced::widget::Id::new("input_panel_acp_selector_scroll"))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
        ))
        .height(Length::Fixed(ACP_SELECTOR_MAX_HEIGHT)),
    )
    .style(styles::popover_style)
    .padding(8)
    .width(Length::Fixed(180.0))
    .into();

    Some(
        AboveOverlay::new(toggle, popover)
            .show(app.show_acp_popover)
            .gap(6.0)
            .on_close(Message::View(message::ViewMessage::CloseAcpPopover))
            .into(),
    )
}

pub(super) fn acp_selector_tooltip(selected_acp_agent: &str) -> String {
    format!("切换 ACP 后端：{selected_acp_agent}")
}

pub(super) fn dark_tooltip_content<'a>(label: impl Into<String>) -> Element<'a, Message> {
    container(
        text(label.into())
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .style(styles::tooltip_dark_style)
    .padding([6, 8])
    .into()
}

pub(super) fn skill_title(app: &App, skill_id: &str) -> String {
    app.skills_settings
        .catalog
        .iter()
        .find(|skill| skill.id == skill_id)
        .map(|skill| skill.title.clone())
        .unwrap_or_else(|| skill_id.to_string())
}

pub(super) fn summarize_names(names: &[String]) -> String {
    const MAX_VISIBLE_NAMES: usize = 3;
    let mut preview = names.iter().take(MAX_VISIBLE_NAMES).cloned().collect::<Vec<_>>().join("、");
    let hidden_count = names.len().saturating_sub(MAX_VISIBLE_NAMES);
    if hidden_count > 0 {
        preview.push_str(&format!(" +{hidden_count}"));
    }
    preview
}

pub(super) fn selected_context_card<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    let summary = selected_context_summary(app)?;

    let icon: Element<'_, Message> = icon_svg(Icon::Sliders, 14.0)
        .style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().primary.scale_alpha(0.92)),
        })
        .into();
    let content = row![
        icon,
        column![
            text("已选工具与技能").size(12).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.92)) }
            }),
            text(summary)
                .size(11)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Word)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.68)),
                }),
        ]
        .spacing(2)
        .width(Length::Fill),
        text("调整").size(11).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.palette().primary.scale_alpha(0.88)),
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    Some(
        container(
            button(content)
                .padding([8, 10])
                .width(Length::Fill)
                .style(styles::manual_context_card_button_style)
                .on_press(Message::View(message::ViewMessage::ToggleSessionToolSelectorPopover)),
        )
        .padding(0)
        .width(Length::Fill)
        .style(styles::manual_context_card_style)
        .into(),
    )
}

pub(super) fn selected_context_summary(app: &App) -> Option<String> {
    let Some(runtime) = app.current_session_runtime_ref() else {
        return None;
    };

    let selected_tools = runtime.tool_selector.selected_manual_tools();
    let selected_skills = runtime.tool_selector.selected_manual_skills();
    let selected_shiju_skills = runtime.tool_selector.selected_shiju_skills();
    if !runtime.tool_selector.has_manual_context_selection() {
        return None;
    }

    let tool_names =
        selected_tools.iter().map(|tool_id| tool_display_name(tool_id)).collect::<Vec<_>>();
    let skill_names =
        selected_skills.iter().map(|skill_id| skill_title(app, skill_id)).collect::<Vec<_>>();

    let mut lines = Vec::new();
    if !tool_names.is_empty() {
        lines.push(format!("工具 {}：{}", tool_names.len(), summarize_names(&tool_names)));
    }
    if !skill_names.is_empty() {
        lines.push(format!("技能 {}：{}", skill_names.len(), summarize_names(&skill_names)));
    }
    if !selected_shiju_skills.is_empty() {
        let names = selected_shiju_skills
            .iter()
            .map(|selection| selection.display_name().to_string())
            .collect::<Vec<_>>();
        lines.push(format!("识矩 {}：{}", names.len(), summarize_names(&names)));
    }

    Some(lines.join("  /  "))
}

/// 构建并渲染输入面板的完整UI视图
///
/// 这是输入面板模块的主入口函数，负责组装所有子组件并构建最终的用户界面。
/// 该函数会根据当前应用状态动态生成不同的UI布局，包括：
///
/// - 普通输入模式：标准输入框和控制按钮
/// - 任务模式：带有任务配置表单的高级模式
/// - 队列模式：显示待处理消息队列
/// - TODO模式：当有待办事项时显示TODO面板
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，包含所有必要的UI状态信息
///
/// # 返回值
///
/// 返回一个 `Element<'a, Message>` 类型的UI元素，代表完整的输入面板组件
///
/// # 布局结构
///
/// 输入面板的典型布局从上到下包括：
/// 1. 队列面板（如果有待处理消息）
/// 2. 文件引用列表（如果输入中包含文件引用）
/// 3. 任务模式表单（如果启用了任务模式）
/// 4. 输入编辑器
/// 5. 底部控制栏（模型选择、使用量、附件、发送/取消按钮）
/// 6. TODO面板（如果有待办事项）
///
/// # 示例
///
/// ```rust,ignore
/// let input_panel = input_panel::view(&app);
/// // 将 input_panel 添加到更大的UI布局中
/// ```
pub fn view<'a>(app: &'a App) -> Element<'a, Message> {
    // ========== 从应用状态中提取当前会话的运行时信息 ==========
    // 获取当前会话的运行时引用，如果不存在则使用 None
    let runtime = app.current_session_runtime_ref();

    // 提取运行时状态，如果没有运行时则使用默认值
    // 这些状态控制UI的显示和交互行为
    let is_requesting = runtime.map(|r| r.is_requesting).unwrap_or(false);
    let submit_anim = runtime.map(|r| r.submit_anim).unwrap_or(0);
    let queue = runtime.map(|r| r.queue.clone()).unwrap_or_default();
    let auto_model = runtime.map(|r| r.auto_model).unwrap_or(app.auto_model);
    let model = runtime.map(|r| r.model.clone()).unwrap_or_else(|| app.model.clone());
    let input_editor = runtime.map(|r| &r.input_editor).unwrap_or(&app.input_editor);
    let selected_acp_agent =
        runtime.and_then(|r| r.acp_agent.clone()).or_else(|| app.acp_agent.clone());
    let reasoning_effort = runtime.map(|r| r.reasoning_effort).unwrap_or(app.reasoning_effort);
    let full_access_enabled = runtime.map(|r| r.full_access_enabled).unwrap_or(false);
    let workflow_mode_enabled = runtime.map(|r| r.workflow_mode_enabled).unwrap_or(false);
    let subagent_mode_enabled = runtime.map(|r| r.subagent_mode_enabled).unwrap_or(false);

    // 任务模式相关状态
    let task_mode_enabled = runtime.map(|r| r.task_mode_enabled).unwrap_or(false);
    let task_mode_priority = runtime.map(|r| r.task_mode_priority.as_str()).unwrap_or("999");
    let task_mode_model = runtime.map(|r| r.task_mode_model.as_str()).unwrap_or("auto");
    let task_mode_task_split_mode = runtime
        .map(|r| r.task_mode_task_split_mode)
        .unwrap_or(crate::app::task::TaskSplitMode::FollowSystem);
    let task_mode_executor = runtime.and_then(|r| r.task_mode_executor.clone());
    let task_mode_reasoning_effort = runtime.and_then(|r| r.task_mode_reasoning_effort);
    let task_mode_subtasks = runtime.map(|r| r.task_mode_subtasks.clone()).unwrap_or_default();
    let has_permission_context = app
        .active_session_id
        .as_deref()
        .and_then(|session_id| app.known_session_directory(session_id))
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| app.project_path.clone().filter(|directory| !directory.trim().is_empty()))
        .is_some();

    // ========== 构建模型选择按钮和弹出框 ==========
    let model_toggle =
        model_popover::model_toggle_button(app, auto_model, &model, app.show_model_popover);

    // 构建输入编辑器组件
    let (input, _editor_height) =
        input_editor::build_input_editor(app, input_editor, is_requesting, task_mode_enabled);

    // 构建模型选择弹出框内容
    let model_pop_content: Element<'_, Message> =
        model_popover::model_popover_content(app, auto_model, &model, task_mode_enabled);

    // 组合模型按钮和弹出框为覆盖层组件
    let model_btn: Element<'_, Message> = AboveOverlay::new(model_toggle, model_pop_content)
        .show(app.show_model_popover)
        .gap(6.0)
        .on_close(Message::View(message::ViewMessage::CloseModelPopover))
        .into();

    // 构建附件按钮
    let attach_btn = attachment::attachment_button(app);
    let (agent_tab_btn, tools_tab_btn, skills_tab_btn) = session_tool_tab_buttons(app);
    let acp_btn =
        acp_selector_button(app, selected_acp_agent.clone(), app.current_session_has_started());
    let reasoning_btn =
        reasoning_effort_selector(app, selected_acp_agent.as_deref(), reasoning_effort);
    let permission_btn = Some(full_access_button(has_permission_context, full_access_enabled));
    let workflow_btn = workflow_mode_button(workflow_mode_enabled);
    let subagent_btn = subagent_mode_button(subagent_mode_enabled);

    // ========== 计算按钮启用状态 ==========
    let input_text = input_editor.text().to_string();
    let has_text = !input_text.trim().is_empty();
    let has_attachments = !app.files.is_empty();
    let enabled = has_text || has_attachments;
    let can_send = enabled;

    // 构建使用量按钮
    let usage_btn = usage_button::usage_button(app);

    // 构建任务池按钮（仅在任务模式启用时显示）
    let pool_btn = if task_mode_enabled {
        Some(pool_button(
            enabled,
            task_mode_enabled,
            input_text.clone(),
            task_mode_priority.to_string(),
            task_mode_model.to_string(),
            task_mode_reasoning_effort,
            task_mode_task_split_mode,
            task_mode_subtasks,
        ))
    } else {
        None
    };

    // 构建发送按钮
    let send = send_button(app, enabled, can_send, is_requesting);

    // 如果正在请求中，显示取消按钮
    let cancel = if is_requesting { Some(cancel_button(submit_anim)) } else { None };

    // 组装底部控制栏（包含模型选择、使用量、附件、任务池、取消、发送按钮）
    let bottom_bar = bottom_bar(
        Some(agent_tab_btn),
        Some(tools_tab_btn),
        Some(skills_tab_btn),
        acp_btn,
        reasoning_btn,
        permission_btn,
        workflow_btn,
        subagent_btn,
        model_btn,
        usage_btn,
        attach_btn,
        pool_btn,
        cancel,
        Some(send),
        task_mode_enabled,
    );

    // 占位分隔符
    let divider = Space::new().height(Length::Fixed(0.0));

    // 构建任务模式配置表单（仅在任务模式启用且有运行时时显示）
    let task_mode_form: Element<'_, Message> = if let Some(runtime) = runtime {
        task_mode_form(
            app,
            Some(runtime),
            task_mode_enabled,
            task_mode_priority,
            task_mode_model,
            task_mode_task_split_mode,
            task_mode_executor,
            task_mode_reasoning_effort,
            runtime.task_mode_subtasks.as_slice(),
        )
    } else {
        Space::new().into()
    };

    // 提取并渲染输入文本中的文件引用
    let file_mentions = extract_file_mentions(&input_text);

    let file_refs: Element<'_, Message> =
        render_file_references(&file_mentions, app.file_ref_hovered_index);
    let attachment_strip = attachment::attachment_strip(app);

    // 读取当前会话的TODO项
    let todo_items: &[vw_shared::todo::Todo] = if app.chat_todo_session_id == app.active_session_id
    {
        app.chat_todo_items.as_slice()
    } else {
        &[]
    };

    // ========== 构建主要内容区域 ==========
    // 根据不同状态选择不同的布局：
    // 1. 有TODO项：显示TODO面板
    // 2. 队列为空或任务模式启用：标准布局
    // 3. 队列非空且非任务模式：显示队列面板
    let show_input_todo =
        !todo_items.is_empty() && app.chat_todo_placement == TodoPanelPlacement::InputBottom;

    let content = if show_input_todo {
        // 有待办事项时，将 TODO 面板紧贴输入框上方
        let todo_panel = todo_panel::todo_panel(
            app,
            todo_items,
            submit_anim,
            todo_panel::TodoPanelSurface::InputBottom,
        );
        let mut input_block = column![todo_panel].spacing(0);
        if let Some(context_card) = selected_context_card(app) {
            input_block = input_block.push(context_card);
        }
        input_block = input_block.push(input).push(divider).push(bottom_bar);

        column![attachment_strip, file_refs, task_mode_form, input_block].spacing(3)
    } else if queue.is_empty() || task_mode_enabled {
        // 标准布局：文件引用 + 任务模式表单 + 输入框 + 底部栏
        let mut standard = column![attachment_strip, file_refs, task_mode_form].spacing(6);
        if let Some(context_card) = selected_context_card(app) {
            standard = standard.push(context_card);
        }
        standard.push(input).push(divider).push(bottom_bar)
    } else {
        // 队列布局：队列面板 + 标准内容
        let queue = queue_panel::queue_panel(queue, is_requesting);

        let mut queued = column![
            queue,
            Space::new().height(Length::Fixed(4.0)),
            attachment_strip,
            file_refs,
            task_mode_form
        ]
        .spacing(6);
        if let Some(context_card) = selected_context_card(app) {
            queued = queued.push(context_card);
        }
        queued.push(input).push(divider).push(bottom_bar)
    };

    // 构建文件搜索覆盖层
    let file_search_content: Element<'_, Message> = file_search::file_search_overlay(app);
    let slash_palette_content: Element<'_, Message> = slash_palette::slash_palette_overlay(app);

    // 将文件搜索作为覆盖层添加到主内容上
    let content_with_file_search: Element<'_, Message> =
        AboveOverlay::new(content, file_search_content)
            .show(app.show_file_search)
            .gap(6.0)
            .on_close(Message::Chat(message::ChatMessage::FileSearchInputChanged(String::new())))
            .into();

    // Slash 面板与 @ 文件搜索互斥显示
    let content_with_overlays: Element<'_, Message> =
        AboveOverlay::new(content_with_file_search, slash_palette_content)
            .show(app.show_slash_palette)
            .gap(6.0)
            .on_close(Message::Chat(message::ChatMessage::SlashPaletteClosed))
            .into();

    // 检测是否有文件树项正在被拖拽
    let dragging_any_tree_item =
        !app.dragging_file_paths.is_empty() || !app.pending_drop_file_paths.is_empty();

    // 包装主内容为拖放区域，支持文件拖拽添加
    let drop_area: Element<'_, Message> = Element::new(DropArea::new(
        content_with_overlays,
        Message::Chat(message::ChatMessage::InputAreaDragDrop),
        Some((
            Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(true)),
            Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(false)),
        )),
        dragging_any_tree_item,
    ));

    // 检测输入区域是否处于拖拽悬停状态
    let input_drop_hovered = app.input_drop_hovered && dragging_any_tree_item;
    let chrome = crate::app::theme_catalog::resolve_chrome_style_from_skin(
        app.active_theme_skin.as_ref(),
        &app.app_theme,
    );

    // 应用容器样式并返回最终UI元素
    let card = container(drop_area)
        .style(move |theme: &iced::Theme| {
            styles::input_card_style(theme, input_drop_hovered, Some(&chrome))
        })
        .padding(0)
        .width(Length::Fill);

    let input_bg = app.active_theme_skin.as_ref().and_then(|skin| skin.chat_input_path.as_deref());
    let fit = app
        .active_theme_skin
        .as_ref()
        .map(|skin| {
            crate::app::theme_catalog::PanelImageFit::parse(
                skin.chrome.chat_input_image_fit.as_deref(),
            )
            .unwrap_or_default()
        })
        .unwrap_or_default();
    let card = with_panel_background(card.into(), input_bg, fit);
    let running_agent_sessions = agent_sessions::running_agent_sessions(app);
    let mut panel = column![].spacing(6).width(Length::Fill);
    if let Some(entry) = agent_sessions::agent_sessions_entry(app, &running_agent_sessions) {
        panel = panel.push(entry);
    }
    if let Some(quick_messages) = quick_messages::view(app) {
        panel = panel.push(quick_messages);
    }
    conversation_reading_column(
        panel.push(card),
        Padding { top: 0.0, right: 2.0, bottom: 0.0, left: 2.0 },
    )
    .into()
}
