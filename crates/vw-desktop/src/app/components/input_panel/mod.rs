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
pub(crate) mod acp_options;
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
#[cfg(not(target_arch = "wasm32"))]
mod voice_controls;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod voice_controls_tests;

pub(crate) const MODEL_POPOVER_SCROLL_ID: &str = "input_panel_model_popover_scroll";

#[cfg(test)]
#[path = "agent_sessions_tests.rs"]
mod agent_sessions_tests;
#[cfg(test)]
#[path = "drop_area_tests.rs"]
mod drop_area_tests;
#[cfg(test)]
#[path = "file_search_tests.rs"]
mod file_search_tests;
#[cfg(test)]
#[path = "input_editor_tests.rs"]
mod input_editor_tests;
#[cfg(test)]
#[path = "model_popover_tests.rs"]
mod model_popover_tests;
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

use crate::app::components::chat_panel::{
    conversation_reading_column, tool_selector::session_tool_selector_popover,
};
use crate::app::components::overlays::AboveOverlay;

use crate::app::theme_catalog::with_panel_background;
use crate::app::{App, Message, TodoPanelPlacement, message};
use drop_area::DropArea;
use file_references::{extract_file_mentions, render_file_references};
use iced::widget::{Space, column, container};
use iced::{Element, Length, Padding};
use reasoning_selector::reasoning_effort_selector;
use send_controls::{
    bottom_bar, cancel_button, full_access_button, pool_button, send_button, workflow_mode_button,
};
use task_mode::task_mode_form;

pub(super) fn session_tool_tab_button<'a>(
    app: &'a App,
    tab: crate::app::state::SessionToolSelectorTab,
) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    let active_tab = runtime.visible_session_tool_selector_tab();
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

    let key = runtime.agent.as_deref().unwrap_or(crate::app::state::MAIN_AGENT_KEY);
    let label = runtime.agent_label.clone().or_else(|| app.agents_settings.entries.iter()
        .find(|entry| entry.key == key).map(|entry| entry.label.clone())).unwrap_or_else(|| key.to_string());
    let toggle = desktop_chat_input::input_panel::panel::session_tool_tab_toggle(
        tab, highlight, label, runtime.acp_agent.as_deref(),
        Message::Chat(message::ChatMessage::OpenSessionToolSelectorTab(tab)),
    );

    if popover_open && active_tab == tab {
        AboveOverlay::new(toggle, session_tool_selector_popover(app))
            .show(true)
            .block_background_events(true)
            .gap(6.0)
            .on_close(Message::View(message::ViewMessage::ClosePopovers))
            .into()
    } else {
        toggle
    }
}

pub(super) fn session_tool_tab_buttons<'a>(
    app: &'a App,
) -> (Element<'a, Message>, Option<Element<'a, Message>>, Element<'a, Message>) {
    let show_tools =
        app.current_session_runtime_ref().is_none_or(|runtime| runtime.acp_agent.is_none());
    (
        session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Agent),
        show_tools.then(|| {
            session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Tools)
        }),
        session_tool_tab_button(app, crate::app::state::SessionToolSelectorTab::Skills),
    )
}

pub(super) fn skill_title(app: &App, skill_id: &str) -> String {
    app.skills_settings
        .catalog
        .iter()
        .find(|skill| skill.id == skill_id)
        .map(|skill| skill.title.clone())
        .or_else(|| {
            #[cfg(target_arch = "wasm32")]
            return None;
            #[cfg(not(target_arch = "wasm32"))]
            match &app.expert_hub.public_market.installed {
                crate::app::skill_hub::LoadState::Loaded(items) => {
                    items.iter().find(|item| item.id == skill_id).map(|item| item.title.clone())
                }
                _ => None,
            }
        })
        .unwrap_or_else(|| skill_id.to_string())
}

pub(super) fn selected_context_card(app: &App) -> Option<Element<'_, Message>> {
    let summary = selected_context_summary(app)?;
    let cancel = app.current_session_runtime().tool_selector.selected_manual_skills().into_iter()
        .find(|id| id.contains("--skillset--"))
        .map(|id| Message::Chat(message::ChatMessage::SessionSkillSelected(id)));
    Some(desktop_chat_input::input_panel::panel::selected_context_card(
        crate::app::i18n::I18n::new(app.desktop_locale), summary,
        Message::View(message::ViewMessage::ToggleSessionToolSelectorPopover), cancel,
    ))
}

pub(super) fn selected_context_summary(app: &App) -> Option<String> {
    let Some(runtime) = app.current_session_runtime_ref() else {
        return None;
    };
    if !runtime.tool_selector.has_manual_context_selection() {
        return None;
    }

    let selected_tools = if runtime.acp_agent.is_none() {
        runtime.tool_selector.selected_manual_tools()
    } else {
        Vec::new()
    };
    let selected_skills = runtime.tool_selector.selected_manual_skills();
    let selected_shiju_skills = runtime.tool_selector.selected_shiju_skills();

    let tool_names = selected_tools
        .iter()
        .map(|tool_id| {
            crate::app::i18n::other::tool_name(
                crate::app::i18n::I18n::new(app.desktop_locale),
                tool_id,
            )
        })
        .collect::<Vec<_>>();
    let skill_names =
        selected_skills.iter().map(|skill_id| skill_title(app, skill_id)).collect::<Vec<_>>();

    let shiju_names = selected_shiju_skills.iter().map(|selection| selection.display_name().to_string()).collect::<Vec<_>>();
    desktop_chat_input::input_panel::panel::selected_context_summary(
        crate::app::i18n::I18n::new(app.desktop_locale), &tool_names, &skill_names, &shiju_names,
    )
}

/// 投影桌面状态并装配各领域的输入面板子视图。
pub fn view<'a>(app: &'a App) -> Element<'a, Message> {
    // ========== 从应用状态中提取当前会话的运行时信息 ==========
    // 获取当前会话的运行时引用，如果不存在则使用 None
    let runtime = app.current_session_runtime_ref();

    // 提取运行时状态，如果没有运行时则使用默认值
    // 这些状态控制UI的显示和交互行为
    let is_requesting = runtime.map(|r| r.request_lifecycle.is_requesting).unwrap_or(false);
    let submit_anim = runtime.map(|r| r.request_lifecycle.submit_anim).unwrap_or(0);
    let queue = runtime.map(|r| r.request_lifecycle.queue.clone()).unwrap_or_default();
    let auto_model = runtime.map(|r| r.auto_model).unwrap_or(app.auto_model);
    let model = runtime.map(|r| r.model.clone()).unwrap_or_else(|| app.model.clone());
    let input_editor = runtime.map(|r| &r.input_editor).unwrap_or(&app.input_editor);
    let selected_acp_agent = runtime.and_then(|r| r.acp_agent.clone());
    let reasoning_effort = runtime.map(|r| r.reasoning_effort).unwrap_or(app.reasoning_effort);
    let full_access_enabled = app.current_session_full_access();
    let workflow_mode_enabled = runtime.map(|r| r.workflow_mode_enabled).unwrap_or(false);

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
    let has_permission_context = send_controls::has_permission_context(app);

    // ========== 构建模型选择按钮和弹出框 ==========
    let model_toggle =
        model_popover::model_toggle_button(app, auto_model, &model, app.show_model_popover);

    // 构建输入编辑器组件
    let (input, _editor_height) =
        input_editor::build_input_editor(app, input_editor, is_requesting, task_mode_enabled);

    let input = if matches!(
        app.voice.draft.phase,
        crate::app::message::voice_draft::Phase::Recording
            | crate::app::message::voice_draft::Phase::Transcribing
    ) {
        voice_controls::recording_bar(app)
    } else {
        input
    };

    // 构建模型选择弹出框内容
    let model_pop_content: Element<'_, Message> =
        model_popover::model_popover_content(app, auto_model, &model);

    // 组合模型按钮和弹出框为覆盖层组件
    let model_btn: Element<'_, Message> = AboveOverlay::new(model_toggle, model_pop_content)
        .show(app.show_model_popover)
        .block_background_events(true)
        .gap(6.0)
        .on_close(Message::View(message::ViewMessage::CloseModelPopover))
        .into();

    // 构建附件按钮
    let attach_btn = attachment::attachment_button(app);
    #[cfg(not(target_arch = "wasm32"))]
    let attach_btn = iced::widget::row![attach_btn, voice_controls::view(app)]
        .spacing(3)
        .align_y(iced::Alignment::Center)
        .into();
    let (agent_tab_btn, tools_tab_btn, skills_tab_btn) = session_tool_tab_buttons(app);

    let acp_btn = app.active_session_id.as_ref().and_then(|_| acp_options::entry(app, runtime));
    let reasoning_btn = if selected_acp_agent.is_some() {
        None
    } else {
        reasoning_effort_selector(app, None, reasoning_effort)
    };
    let permission_btn = Some(full_access_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        has_permission_context
            && runtime.is_none_or(|runtime| {
                !runtime.full_access_saving && (runtime.draft.is_none() || !runtime.request_lifecycle.is_requesting)
            }),
        full_access_enabled,
    ));
    let workflow_btn = workflow_mode_button(
        crate::app::i18n::I18n::new(app.desktop_locale),
        workflow_mode_enabled,
    );

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
            crate::app::i18n::I18n::new(app.desktop_locale),
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
    let cancel = if is_requesting {
        Some(cancel_button(crate::app::i18n::I18n::new(app.desktop_locale), submit_anim))
    } else {
        None
    };

    // 组装底部控制栏（包含模型选择、使用量、附件、任务池、取消、发送按钮）
    let bottom_bar = bottom_bar(
        Some(agent_tab_btn),
        tools_tab_btn,
        Some(skills_tab_btn),
        acp_btn,
        reasoning_btn,
        permission_btn,
        workflow_btn,
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

    let file_refs: Element<'_, Message> = render_file_references(
        crate::app::i18n::I18n::new(app.desktop_locale),
        &file_mentions,
        app.file_ref_hovered_index,
    );
    let attachment_strip = attachment::attachment_strip(app);

    // 读取当前会话的TODO项
    let todo_items: &[todo_types::Todo] = if app.chat_todo_session_id == app.active_session_id
    {
        app.chat_todo_items.as_slice()
    } else {
        &[]
    };

    let show_input_todo = !todo_items.is_empty() && app.chat_todo_placement == TodoPanelPlacement::InputBottom;
    let todo_panel = show_input_todo.then(|| todo_panel::todo_panel(
        app, todo_items, submit_anim, todo_panel::TodoPanelSurface::InputBottom,
    ));
    let queue_panel = (!show_input_todo && !queue.is_empty() && !task_mode_enabled).then(||
        queue_panel::queue_panel(crate::app::i18n::I18n::new(app.desktop_locale), queue, is_requesting));
    let content = desktop_chat_input::input_panel::layout::content(
        desktop_chat_input::input_panel::layout::Content {
            attachment_strip, file_refs, task_mode_form, input, divider, bottom_bar,
            context_card: selected_context_card(app), todo_panel, queue_panel,
        },
    );

    // 构建文件搜索覆盖层
    let file_search_content: Element<'_, Message> = if app.show_file_search {
        file_search::file_search_overlay(app)
    } else {
        Space::new().into()
    };
    let slash_palette_content: Element<'_, Message> = if app.show_slash_palette {
        slash_palette::slash_palette_overlay(app)
    } else {
        Space::new().into()
    };

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
        !app.file_tree_interaction.dragging_file_paths.is_empty() || !app.file_tree_interaction.pending_drop_file_paths.is_empty();

    // 包装主内容为拖放区域，支持文件拖拽添加
    let drop_area: Element<'_, Message> = Element::new(
        DropArea::new(
            content_with_overlays,
            Message::Chat(message::ChatMessage::InputAreaDragDrop),
            Some((
                Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(true)),
                Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(false)),
            )),
            dragging_any_tree_item,
        )
        .on_native_drop(|path| {
            Message::View(message::ViewMessage::DroppedFilePath(
                path.to_string_lossy().into_owned(),
            ))
        }),
    );

    // 检测输入区域是否处于拖拽悬停状态
    let input_drop_hovered = app.file_tree_interaction.input_drop_hovered && dragging_any_tree_item;
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
    let running_agent_sessions = agent_sessions::cached_running_agent_sessions(app);
    let mut panel = column![].spacing(6).width(Length::Fill);
    if let Some(voice) = super::voice_chat::local_inline(app) {
        panel = panel.push(voice);
    }
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

fn slice_message(message: desktop_chat_input::input_panel::slice_message::Message) -> Message {
    use desktop_chat_input::input_panel::slice_message::Message as Local;
    Message::Chat(match message {
        Local::RemoveFileReference(v) => message::ChatMessage::RemoveFileReference(v),
        Local::FileReferenceHoverChanged(v) => message::ChatMessage::FileReferenceHoverChanged(v),
        Local::FileSearchSelect(v) => message::ChatMessage::FileSearchSelect(v),
        Local::SlashPaletteNavigateUp => message::ChatMessage::SlashPaletteNavigateUp,
        Local::SlashPaletteNavigateDown => message::ChatMessage::SlashPaletteNavigateDown,
        Local::SlashPaletteSelectCurrent => message::ChatMessage::SlashPaletteSelectCurrent,
        Local::SlashPaletteClosed => message::ChatMessage::SlashPaletteClosed,
        Local::FileSearchNavigateUp => message::ChatMessage::FileSearchNavigateUp,
        Local::FileSearchNavigateDown => message::ChatMessage::FileSearchNavigateDown,
        Local::FileSearchSelectCurrent => message::ChatMessage::FileSearchSelectCurrent,
        Local::FileSearchInputChanged(v) => message::ChatMessage::FileSearchInputChanged(v),
        Local::PasteIntoInput => message::ChatMessage::PasteIntoInput,
        Local::SendPressed => message::ChatMessage::SendPressed,
        Local::InputEditorAction(v) => message::ChatMessage::InputEditorAction(v),
        Local::OpenInputContextMenu { x, y } => message::ChatMessage::OpenInputContextMenu { x, y },
        Local::CloseInputContextMenu => message::ChatMessage::CloseInputContextMenu,
        Local::CopyInputSelection => message::ChatMessage::CopyInputSelection,
        Local::CutInputSelection => message::ChatMessage::CutInputSelection,
        Local::SelectAllInput => message::ChatMessage::SelectAllInput,
        Local::QueueGuide(v) => message::ChatMessage::QueueGuide(v),
        Local::QueueEdit(v) => message::ChatMessage::QueueEdit(v),
        Local::QueueCopy(v) => message::ChatMessage::QueueCopy(v),
        Local::QueueUp(v) => message::ChatMessage::QueueUp(v),
        Local::QueueDown(v) => message::ChatMessage::QueueDown(v),
        Local::QueueRemove(v) => message::ChatMessage::QueueRemove(v),
    })
}
