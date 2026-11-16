//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, focus_input_editor, sync_global_input_editor_if_needed,
};
use crate::app::components::input_mention_highlighter::{
    file_mention, mention_ranges,
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
    match act {
        text_editor::Action::Edit(edit) => handle_input_editor_edit(app, edit),
        other => handle_input_editor_non_edit(app, other),
    }
}

/// 模块内可见函数，执行 handle_file_search_input_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_input_changed(app: &mut App, value: String) -> Task<Message> {
    let query_changed = app.file_search_query != value;
    app.file_search_query = value.clone();
    app.refresh_file_search_cache();

    if value.trim().is_empty() {
        app.show_file_search = false;
        app.file_search_selected_index = 0;
    } else {
        app.show_file_search = true;
        app.file_search_selected_index = 0;
    }

    let focus_task = focus_input_editor(app);
    if app.show_file_search && query_changed {
        Task::batch(vec![focus_task, scroll_file_search_to_top(app)])
    } else {
        focus_task
    }
}

/// 模块内可见函数，执行 handle_file_search_navigate_up 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_navigate_up(app: &mut App) -> Task<Message> {
    if app.file_search_selected_index > 0 {
        app.file_search_selected_index -= 1;
    }
    let total = ranked_file_search_results(app).len().min(20);
    scroll_file_search_to_selected(app, total)
}

/// 模块内可见函数，执行 handle_file_search_navigate_down 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_navigate_down(app: &mut App) -> Task<Message> {
    let files = ranked_file_search_results(app);
    if app.file_search_selected_index < files.len().min(20).saturating_sub(1) {
        app.file_search_selected_index += 1;
    }
    let total = files.len().min(20);
    scroll_file_search_to_selected(app, total)
}

/// 模块内可见函数，执行 handle_file_search_select_current 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_file_search_select_current(app: &mut App) -> Task<Message> {
    let files = ranked_file_search_results(app);
    if let Some(path) = files.get(app.file_search_selected_index) {
        return select_file_search_path(app, path);
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
    let runtime = app.current_session_runtime();
    let text = runtime.input_editor.text().to_string();
    let mention_pattern = file_mention(&file_path);

    let mut new_text = text.clone();
    for mut range in mention_ranges(&text).into_iter().rev() {
        if text[range.clone()] != mention_pattern {
            continue;
        }
        // 删除引用后的分隔空白，保留其它文件名内部的空格。
        range.end += text[range.end..]
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
        new_text.replace_range(range, "");
    }
    let new_text = new_text.trim();

    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(new_text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));

    sync_global_input_editor_if_needed(app);
    app.file_ref_hovered_index = None;
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
            &join_drop_mentions(&mentions),
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

fn handle_input_editor_edit(app: &mut App, edit: text_editor::Edit) -> Task<Message> {
    app.focus_area = FocusArea::None;
    close_input_context_menu(app);

    {
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor.perform(text_editor::Action::Edit(edit));
    }

    sync_global_input_editor_if_needed(app);

    let text = app.current_session_runtime_mut().input_editor.text();
    let previous_file_query = app.file_search_query.clone();
    let previous_slash_query = app.slash_palette_query.clone();
    let last_char = text.chars().last();

    if last_char.is_some_and(|c: char| c.is_whitespace()) {
        clear_file_search_state(app);
        super::slash_palette::clear_slash_palette_state(app);
        return Task::none();
    }

    if let Some(query) = super::slash_palette::active_slash_query(&text) {
        return super::slash_palette::open_slash_palette(app, query);
    }

    if let Some(query) = active_file_search_query(&text) {
        super::slash_palette::clear_slash_palette_state(app);
        app.show_file_search = true;
        app.file_search_query = query.to_string();
        app.refresh_file_search_cache();
        app.file_search_selected_index = 0;
        let focus_task = focus_input_editor(app);
        if app.file_search_query != previous_file_query
            || previous_slash_query != app.slash_palette_query
        {
            return Task::batch(vec![focus_task, scroll_file_search_to_top(app)]);
        }
        return focus_task;
    }

    clear_file_search_state(app);
    super::slash_palette::clear_slash_palette_state(app);
    Task::none()
}

fn handle_input_editor_non_edit(app: &mut App, action: text_editor::Action) -> Task<Message> {
    app.focus_area = FocusArea::None;
    close_input_context_menu(app);
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor.perform(action);
    sync_global_input_editor_if_needed(app);
    Task::none()
}

fn select_file_search_path(app: &mut App, path: &str) -> Task<Message> {
    let rel_or_full_path = if let Some(project_root) = &app.navigation_tabs.project_path {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|entry| entry.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    };
    let replacement = file_mention(&rel_or_full_path);

    let runtime = app.current_session_runtime();
    let text = replace_or_append_file_mention(&runtime.input_editor.text(), &replacement);
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(&text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));

    sync_global_input_editor_if_needed(app);
    clear_file_search_state(app);
    app.show_file_search = false;
    app.focus_area = FocusArea::None;
    focus_input_editor(app)
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

fn clear_file_search_state(app: &mut App) {
    app.show_file_search = false;
    app.file_search_query.clear();
    app.refresh_file_search_cache();
    app.file_search_selected_index = 0;
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
        (app.file_search_selected_index.min(total.saturating_sub(1)) as f32) / ((total - 1) as f32)
    };

    operation::snap_to(
        app.file_search_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(y) },
    )
    .map(|_: ()| Message::None)
}

pub(crate) use desktop_session::message::chat::input::file_search::*;
