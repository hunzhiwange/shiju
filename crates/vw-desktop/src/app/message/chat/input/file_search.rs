//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, focus_input_editor, sync_global_input_editor_if_needed,
};
use crate::app::{App, FocusArea, Message};
use iced::{Task, widget::operation, widget::text_editor};

/// 模块内可见函数，执行 ranked_file_search_results 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn ranked_file_search_results(app: &App) -> Vec<String> {
    ranked_file_search_entries(app).into_iter().map(|entry| entry.path).collect()
}

/// 模块内可见函数，执行 ranked_file_search_entries 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn ranked_file_search_entries(app: &App) -> Vec<FileSearchResult> {
    app.cached_file_search_entries().to_vec()
}

/// 模块内可见函数，执行 handle_input_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_input_editor_action(app: &mut App, act: text_editor::Action) -> Task<Message> {
    app.focus_area = FocusArea::None;
    close_input_context_menu(app);
    let mut state = std::mem::take(&mut app.input_suggestions);
    let empty = app.skills_settings.catalog.is_empty();
    let loading = app.skills_settings.loading;
    let effects = state.editor_action(&mut app.current_session_runtime_mut().input_editor, act, empty, loading);
    app.input_suggestions = state;
    sync_global_input_editor_if_needed(app);
    apply_suggestion_effects(app, effects)
}

/// 模块内可见函数，执行 handle_file_search_input_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_input_changed(app: &mut App, value: String) -> Task<Message> {
    let effects = app.input_suggestions.change_files(value);
    apply_suggestion_effects(app, effects)
}

/// 模块内可见函数，执行 handle_file_search_navigate_up 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_navigate_up(app: &mut App) -> Task<Message> {
    let total = ranked_file_search_results(app).len().min(20);
    app.input_suggestions.navigate_file(false, total);
    scroll_file_search_to_selected(app, total)
}

/// 模块内可见函数，执行 handle_file_search_navigate_down 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_navigate_down(app: &mut App) -> Task<Message> {
    let total = ranked_file_search_results(app).len().min(20);
    app.input_suggestions.navigate_file(true, total);
    scroll_file_search_to_selected(app, total)
}

/// 模块内可见函数，执行 handle_file_search_select_current 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_select_current(app: &mut App) -> Task<Message> {
    if let Some(path) = app.input_suggestions.current_file() {
        return select_file_search_path(app, &path);
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_file_search_select 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_select(app: &mut App, path: String) -> Task<Message> {
    select_file_search_path(app, &path)
}

/// 模块内可见函数，执行 handle_remove_file_reference 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_remove_file_reference(app: &mut App, file_path: String) -> Task<Message> {
    desktop_chat_input::input_panel::file_references::remove_file_reference(
        &mut app.current_session_runtime_mut().input_editor, &file_path,
    );
    sync_global_input_editor_if_needed(app);
    app.input_suggestions.file_ref_hovered_index = None;
    Task::none()
}

/// 模块内可见函数，执行 handle_input_area_drag_drop 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_input_area_drag_drop(app: &mut App) -> Task<Message> {
    app.file_tree_interaction.input_drop_hovered = false;
    let mentions = drop_mention_paths(app);
    if !mentions.is_empty() {
        let runtime = app.current_session_runtime_mut();
        crate::app::ui::chat::insert_at_cursor(
            &mut runtime.input_editor,
            &desktop_session::message::chat::input::file_search::join_drop_mentions(&mentions),
        );

        sync_global_input_editor_if_needed(app);
        clear_drag_drop_state(app);
        app.focus_area = FocusArea::None;
        return focus_input_editor(app);
    }

    app.file_tree_interaction.pending_drop_file_paths.clear();
    app.file_tree_interaction.pending_drop_file_position = None;
    Task::none()
}

fn select_file_search_path(app: &mut App, path: &str) -> Task<Message> {
    let root = app.navigation_tabs.project_path.clone();
    let mut state = std::mem::take(&mut app.input_suggestions);
    let effects = state.select_file(&mut app.current_session_runtime_mut().input_editor, root.as_deref(), path);
    app.input_suggestions = state;
    sync_global_input_editor_if_needed(app);
    app.focus_area = FocusArea::None;
    apply_suggestion_effects(app, effects)
}

fn drop_mention_paths(app: &App) -> Vec<String> {
    let dropped_paths = collect_dropped_paths(app);
    if dropped_paths.is_empty() {
        return Vec::new();
    }

    let drop_position = if dropped_paths.len() == 1 {
        app.file_tree_interaction.dragging_file_position.or(app.file_tree_interaction.pending_drop_file_position)
    } else {
        None
    };

    format_drop_mentions(app.navigation_tabs.project_path.as_deref(), &dropped_paths, drop_position)
}

fn collect_dropped_paths(app: &App) -> Vec<DroppedPath> {
    let source_paths = if !app.file_tree_interaction.dragging_file_paths.is_empty() {
        &app.file_tree_interaction.dragging_file_paths
    } else {
        &app.file_tree_interaction.pending_drop_file_paths
    };

    source_paths
        .iter()
        .map(|path| DroppedPath { path: path.clone(), is_dir: std::path::Path::new(path).is_dir() })
        .collect()
}

fn clear_drag_drop_state(app: &mut App) {
    app.file_tree_interaction.dragging_file_paths.clear();
    app.file_tree_interaction.dragging_file_position = None;
    app.file_tree_interaction.pending_drop_file_paths.clear();
    app.file_tree_interaction.pending_drop_file_position = None;
}

fn scroll_file_search_to_top(app: &App) -> Task<Message> {
    iced::widget::operation::snap_to(
        app.file_search_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
    )
    .map(|_: ()| Message::None)
}

fn scroll_file_search_to_selected(app: &App, total: usize) -> Task<Message> {
    if total == 0 {
        return Task::none();
    }

    let y = if total <= 1 {
        0.0
    } else {
        (app.input_suggestions.file_search_selected_index.min(total.saturating_sub(1)) as f32) / ((total - 1) as f32)
    };

    operation::snap_to(
        app.file_search_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(y) },
    )
    .map(|_: ()| Message::None)
}

pub(crate) use desktop_session::message::chat::input::file_search::*;

/// 将局部转换结果映射到索引与 Iced 生命周期操作。
pub(super) fn apply_suggestion_effects(app: &mut App, effects: desktop_chat_input::input_panel::suggestions::Effects) -> Task<Message> {
    let mut tasks = Vec::new();
    if effects.refresh_files { app.refresh_file_search_cache(); }
    if effects.focus { tasks.push(focus_input_editor(app)); }
    if effects.scroll_files_to_top { tasks.push(scroll_file_search_to_top(app)); }
    if effects.scroll_slash_to_top { tasks.push(super::slash_palette::scroll_slash_palette_to_top(app)); }
    if effects.refresh_skills { tasks.push(Task::done(Message::Settings(crate::app::message::SettingsMessage::SkillsRefresh))); }
    Task::batch(tasks)
}
