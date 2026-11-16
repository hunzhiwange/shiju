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
mod tests;
#[cfg(test)]
#[path = "todo_panel_tests.rs"]
mod todo_panel_tests;

use crate::app::components::chat_panel::{
    tool_selector::session_tool_selector_popover,
};
use crate::app::components::overlays::AboveOverlay;

use crate::app::{App, Message, TodoPanelPlacement, message};
use file_references::{extract_file_mentions, render_file_references};
use iced::widget::Space;
use iced::{Element, Length};
use reasoning_selector::reasoning_effort_selector;
use send_controls::{
    cancel_button, full_access_button, pool_button, send_button, workflow_mode_button,
};
use task_mode::task_mode_form;

pub(super) fn session_tool_tab_button<'a>(
    app: &'a App,
    tab: crate::app::state::SessionToolSelectorTab,
) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    let active_tab = runtime.visible_session_tool_selector_tab();
    let popover_open = app.popover_visibility.session_tool_selector;
    let highlight = desktop_chat_input::input_panel::panel::session_tool_tab_highlight(
        tab, popover_open, active_tab, runtime.agent.is_some(), &runtime.tool_selector,
    );
    let key = runtime.agent.as_deref().unwrap_or(crate::app::state::MAIN_AGENT_KEY);
    let label = desktop_chat_input::input_panel::panel::agent_label(
        key, runtime.agent_label.as_deref(), app.agents_settings.entries.iter().map(|entry| (entry.key.as_str(), entry.label.as_str())),
    );
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
    let titles = app.skills_settings.catalog.iter().map(|skill| (skill.id.as_str(), skill.title.as_str()));
    #[cfg(not(target_arch = "wasm32"))]
    let titles = {
        let market = match &app.expert_hub.public_market.installed {
            crate::app::skill_hub::LoadState::Loaded(items) => Some(items.as_slice()),
            _ => None,
        };
        titles.chain(market.into_iter().flatten().map(|item| (item.id.as_str(), item.title.as_str())))
    };
    desktop_chat_input::input_panel::panel::skill_title(skill_id, titles)
}

pub(super) fn selected_context_card(app: &App) -> Option<Element<'_, Message>> {
    let summary = selected_context_summary(app)?;
    let cancel = app.current_session_runtime().tool_selector.selected_manual_skills().into_iter()
        .find(|id| id.contains("--skillset--"))
        .map(|id| Message::Chat(message::ChatMessage::SessionSkillSelected(id)));
    Some(desktop_chat_input::input_panel::panel::selected_context_card(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale), summary,
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
                crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
                tool_id,
            )
        })
        .collect::<Vec<_>>();
    let skill_names =
        selected_skills.iter().map(|skill_id| skill_title(app, skill_id)).collect::<Vec<_>>();

    let shiju_names = selected_shiju_skills.iter().map(|selection| selection.display_name().to_string()).collect::<Vec<_>>();
    desktop_chat_input::input_panel::panel::selected_context_summary(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &tool_names, &skill_names, &shiju_names,
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
    let auto_model = runtime.map(|r| r.auto_model).unwrap_or(app.session_state.auto_model);
    let model = runtime.map(|r| r.model.clone()).unwrap_or_else(|| app.session_state.model.clone());
    let input_editor = runtime.map(|r| &r.input_editor).unwrap_or(&app.session_state.input_editor);
    let selected_acp_agent = runtime.and_then(|r| r.acp_agent.clone());
    let reasoning_effort = runtime.map(|r| r.reasoning_effort).unwrap_or(app.session_state.reasoning_effort);
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
        model_popover::model_toggle_button(app, auto_model, &model, app.popover_visibility.model);

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
    let mobile = app.mobile_layout.mode() == crate::app::state::LayoutMode::Mobile;
    let model_btn = desktop_chat_input::input_panel::model_views::with_model_popover(
        model_toggle, model_pop_content, app.popover_visibility.model,
        Message::View(message::ViewMessage::CloseModelPopover),
    );

    // 构建附件按钮
    let attach_btn = attachment::attachment_button(app);
    #[cfg(not(target_arch = "wasm32"))]
    let attach_btn = iced::widget::row![attach_btn, voice_controls::view(app)]
        .spacing(3)
        .align_y(iced::Alignment::Center)
        .into();
    let (agent_tab_btn, tools_tab_btn, skills_tab_btn) = session_tool_tab_buttons(app);

    let acp_btn = app.session_state.active_session_id.as_ref().and_then(|_| acp_options::entry(app, runtime));
    let reasoning_btn = if selected_acp_agent.is_some() {
        None
    } else {
        reasoning_effort_selector(app, None, reasoning_effort)
    };
    let permission_btn = Some(full_access_button(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        desktop_chat_input::input_panel::send_controls::permission_change_allowed(
            has_permission_context, runtime.is_some_and(|r| r.full_access_saving),
            runtime.is_some_and(|r| r.draft.is_some()), runtime.is_some_and(|r| r.request_lifecycle.is_requesting),
        ),
        full_access_enabled,
    ));
    let workflow_btn = workflow_mode_button(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        workflow_mode_enabled,
    );

    // ========== 计算按钮启用状态 ==========
    let input_text = input_editor.text().to_string();
    let enabled = desktop_chat_input::input_panel::send_controls::has_sendable_input(&input_text, !app.session_state.files.is_empty());
    let can_send = enabled;

    // 构建使用量按钮
    let usage_btn = usage_button::usage_button(app);

    // 构建任务池按钮（仅在任务模式启用时显示）
    let pool_btn = if task_mode_enabled {
        Some(pool_button(
            crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
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
        Some(cancel_button(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), submit_anim))
    } else {
        None
    };

    let bottom_bar = desktop_chat_input::input_panel::layout::controls(
        desktop_chat_input::input_panel::layout::Controls {
            agent_tab_btn, skills_tab_btn, workflow_btn, model_btn, usage_btn, attach_btn, send, tools_tab_btn, acp_btn, reasoning_btn, permission_btn, pool_btn, cancel,
            mobile_model_content: if mobile { model_popover::model_popover_content(app, auto_model, &model) } else { Space::new().into() },
        }, mobile,
        app.popover_visibility.model || app.popover_visibility.session_tool_selector
            || app.popover_visibility.acp || app.popover_visibility.reasoning_effort || app.popover_visibility.usage,
        task_mode_enabled,
        Message::View(message::ViewMessage::ToggleModelPopover),
        Message::View(message::ViewMessage::ClosePopovers),
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
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &file_mentions,
        app.input_suggestions.file_ref_hovered_index,
    );
    let attachment_strip = attachment::attachment_strip(app);

    // 读取当前会话的TODO项
    let todo_items: &[todo_types::Todo] = if app.todo_state.chat_todo_session_id == app.session_state.active_session_id
    {
        app.todo_state.chat_todo_items.as_slice()
    } else {
        &[]
    };

    use desktop_chat_input::input_panel::layout::{supplement, Supplement};
    let supplement = supplement(!todo_items.is_empty(), app.todo_state.chat_todo_placement == TodoPanelPlacement::InputBottom, !queue.is_empty(), task_mode_enabled);
    let todo_panel = (supplement == Supplement::Todo).then(|| todo_panel::todo_panel(
        app, todo_items, submit_anim, todo_panel::TodoPanelSurface::InputBottom,
    ));
    let queue_panel = (supplement == Supplement::Queue).then(||
        queue_panel::queue_panel(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), queue, is_requesting));
    let content = desktop_chat_input::input_panel::layout::Content {
        attachment_strip, file_refs, task_mode_form, input, divider, bottom_bar,
        context_card: selected_context_card(app), todo_panel, queue_panel,
    };

    // 构建文件搜索覆盖层
    let file_search_content: Element<'_, Message> = if app.input_suggestions.show_file_search {
        file_search::file_search_overlay(app)
    } else {
        Space::new().into()
    };
    let slash_palette_content: Element<'_, Message> = if app.input_suggestions.show_slash_palette {
        slash_palette::slash_palette_overlay(app)
    } else {
        Space::new().into()
    };

    let chrome = crate::app::theme_catalog::resolve_chrome_style_from_skin(
        app.appearance.active_theme_skin.as_ref(), &app.appearance.app_theme,
    );
    let fit = app.appearance.active_theme_skin.as_ref().and_then(|skin|
        crate::app::theme_catalog::PanelImageFit::parse(skin.chrome.chat_input_image_fit.as_deref())
    ).unwrap_or_default();
    let running_agent_sessions = agent_sessions::cached_running_agent_sessions(app);
    desktop_chat_input::input_panel::layout::view(
        content,
        desktop_chat_input::input_panel::layout::Surface {
            file_search: file_search_content, slash_palette: slash_palette_content,
            show_file_search: app.input_suggestions.show_file_search, show_slash_palette: app.input_suggestions.show_slash_palette,
            dragging: !app.file_tree_interaction.dragging_file_paths.is_empty()
                || !app.file_tree_interaction.pending_drop_file_paths.is_empty(),
            input_drop_hovered: app.file_tree_interaction.input_drop_hovered,
            chrome,
            background: app.appearance.active_theme_skin.as_ref().and_then(|skin| skin.chat_input_path.as_deref()),
            fit,
            voice: super::voice_chat::local_inline(app),
            agent_sessions: agent_sessions::agent_sessions_entry(app, &running_agent_sessions),
            quick_messages: quick_messages::view(app),
        },
        desktop_chat_input::input_panel::layout::SurfaceMessages {
            close_file_search: Message::Chat(message::ChatMessage::FileSearchInputChanged(String::new())),
            close_slash_palette: Message::Chat(message::ChatMessage::SlashPaletteClosed),
            drop: Message::Chat(message::ChatMessage::InputAreaDragDrop),
            hover: (Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(true)),
                    Message::Chat(message::ChatMessage::InputAreaDragHoverChanged(false))),
            native_drop: |path| Message::View(message::ViewMessage::DroppedFilePath(path.to_string_lossy().into_owned())),
        },
    )
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
