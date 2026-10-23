//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, focus_input_editor, sync_global_input_editor_if_needed,
};
use crate::app::{App, FocusArea, Message};
use iced::{Task, widget::operation, widget::text_editor};
use std::collections::BTreeSet;
use std::sync::Arc;

/// 模块内可见结构体，承载 FileSearchResult 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
#[derive(Debug, Clone)]
pub(crate) struct FileSearchResult {
    pub path: String,
    pub is_dir: bool,
}

/// 模块内可见函数，执行 ranked_file_search_results 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn ranked_file_search_results(app: &App) -> Vec<String> {
    ranked_file_search_entries(app).into_iter().map(|entry| entry.path).collect()
}

/// 模块内可见函数，执行 build_ranked_file_search_entries 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(crate) fn build_ranked_file_search_entries(
    file_index: &[String],
    query: &str,
) -> Vec<FileSearchResult> {
    let mut seen_dirs = BTreeSet::new();
    let mut candidates = Vec::new();

    for path in file_index {
        candidates.push(FileSearchResult { path: path.clone(), is_dir: false });

        let normalized = path.replace('\\', "/");
        let mut parts = normalized.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>();
        if parts.len() <= 1 {
            continue;
        }

        parts.pop();
        let mut prefix = String::new();
        for part in parts {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);

            let dir_path = format!("{}/", prefix);
            if seen_dirs.insert(dir_path.clone()) {
                candidates.push(FileSearchResult { path: dir_path, is_dir: true });
            }
        }
    }

    let mut ranked = candidates
        .iter()
        .filter_map(|entry| {
            let score = if query.is_empty() { 0 } else { file_match_score(&entry.path, query)? };
            Some((score, entry.is_dir, entry.path.clone()))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)).then_with(|| a.2.cmp(&b.2)));
    ranked.into_iter().map(|(_, is_dir, path)| FileSearchResult { path, is_dir }).collect()
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

/// 模块内可见函数，执行 wasm IME 提交文本对应的输入流程。
pub(super) fn handle_input_editor_commit(app: &mut App, value: String) -> Task<Message> {
    if value.is_empty() {
        return Task::none();
    }

    handle_input_editor_edit(app, text_editor::Edit::Paste(Arc::new(value)))
}

/// 模块内可见函数，执行 wasm 输入桥 Backspace 对应的输入流程。
pub(super) fn handle_input_editor_backspace(app: &mut App) -> Task<Message> {
    handle_input_editor_edit(app, text_editor::Edit::Backspace)
}

/// 模块内可见函数，执行 wasm 输入桥 Delete 对应的输入流程。
pub(super) fn handle_input_editor_delete(app: &mut App) -> Task<Message> {
    handle_input_editor_edit(app, text_editor::Edit::Delete)
}

/// 模块内可见函数，执行 wasm 输入桥光标移动对应的输入流程。
pub(super) fn handle_input_editor_motion(
    app: &mut App,
    motion: text_editor::Motion,
    select: bool,
) -> Task<Message> {
    if select {
        handle_input_editor_non_edit(app, text_editor::Action::Select(motion))
    } else {
        handle_input_editor_non_edit(app, text_editor::Action::Move(motion))
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
    let mention_pattern = format!("@{}", file_path);

    let new_text = text.replace(&mention_pattern, "");
    let new_text = new_text.split_whitespace().collect::<Vec<_>>().join(" ");

    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(&new_text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));

    sync_global_input_editor_if_needed(app);
    app.file_ref_hovered_index = None;
    Task::none()
}

/// 模块内可见函数，执行 handle_input_area_drag_drop 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_input_area_drag_drop(app: &mut App) -> Task<Message> {
    app.input_drop_hovered = false;
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

    app.pending_drop_file_paths.clear();
    app.pending_drop_file_position = None;
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
    let rel_or_full_path = if let Some(project_root) = &app.project_path {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|entry| entry.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    };
    let replacement = format!("@{}", rel_or_full_path);

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
        app.dragging_file_position.or(app.pending_drop_file_position)
    } else {
        None
    };

    format_drop_mentions(app.project_path.as_deref(), &dropped_paths, drop_position)
}

/// 模块内可见结构体，承载 DroppedPath 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DroppedPath {
    pub(super) path: String,
    pub(super) is_dir: bool,
}

fn collect_dropped_paths(app: &App) -> Vec<DroppedPath> {
    let source_paths = if !app.dragging_file_paths.is_empty() {
        &app.dragging_file_paths
    } else {
        &app.pending_drop_file_paths
    };

    source_paths
        .iter()
        .map(|path| DroppedPath { path: path.clone(), is_dir: std::path::Path::new(path).is_dir() })
        .collect()
}

/// 模块内可见函数，执行 format_drop_mentions 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn format_drop_mentions(
    project_root: Option<&str>,
    dropped_paths: &[DroppedPath],
    dropped_position: Option<(usize, usize)>,
) -> Vec<String> {
    let mention_position = if dropped_paths.len() == 1 { dropped_position } else { None };
    dropped_paths
        .iter()
        .map(|entry| {
            format_drop_mention_path(project_root, &entry.path, entry.is_dir, mention_position)
        })
        .collect()
}

fn format_drop_mention_path(
    project_root: Option<&str>,
    file_path: &str,
    is_dir: bool,
    position: Option<(usize, usize)>,
) -> String {
    let normalized = if let Some(project_root) = project_root
        && let Ok(rel_path) = std::path::Path::new(file_path).strip_prefix(project_root)
    {
        rel_path.to_string_lossy().replace('\\', "/")
    } else {
        file_path.replace('\\', "/")
    };
    let mention_path =
        if is_dir && !normalized.ends_with('/') { format!("{normalized}/") } else { normalized };

    match position {
        Some((line, col)) if !is_dir => format!("{mention_path}:{line}:{col}"),
        _ => mention_path,
    }
}

fn join_drop_mentions(mentions: &[String]) -> String {
    mentions.iter().map(|mention| format!("@{} ", mention)).collect()
}

fn clear_drag_drop_state(app: &mut App) {
    app.dragging_file_paths.clear();
    app.dragging_file_position = None;
    app.pending_drop_file_paths.clear();
    app.pending_drop_file_position = None;
}

fn clear_file_search_state(app: &mut App) {
    app.show_file_search = false;
    app.file_search_query.clear();
    app.refresh_file_search_cache();
    app.file_search_selected_index = 0;
}

pub(super) fn normalized_lower_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub(super) fn fuzzy_subsequence_score(candidate: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let cand = candidate.chars().collect::<Vec<_>>();
    let needle = query.chars().collect::<Vec<_>>();
    let mut score = 0i64;
    let mut cand_i = 0usize;
    let mut prev_match: Option<usize> = None;

    for q in needle {
        let mut found = None;
        while cand_i < cand.len() {
            if cand[cand_i] == q {
                found = Some(cand_i);
                break;
            }
            cand_i += 1;
        }

        let idx = found?;
        score += 10;

        if let Some(prev) = prev_match {
            if idx == prev + 1 {
                score += 8;
            } else {
                score -= (idx.saturating_sub(prev + 1) as i64).min(8);
            }
        }

        if idx == 0 {
            score += 15;
        } else if matches!(cand[idx - 1], '/' | '_' | '-' | '.' | ' ') {
            score += 5;
        }

        prev_match = Some(idx);
        cand_i = idx + 1;
    }

    Some(score)
}

pub(super) fn file_match_score(path: &str, query: &str) -> Option<i64> {
    let normalized = normalized_lower_path(path);
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);

    if query.is_empty() {
        return Some(0);
    }

    let full_subseq = fuzzy_subsequence_score(&normalized, query)?;
    let name_subseq = fuzzy_subsequence_score(file_name, query).unwrap_or(i64::MIN / 2);
    let mut score = full_subseq.max(name_subseq.saturating_add(120));

    if normalized == query {
        score += 2200;
    }
    if file_name == query {
        score += 2000;
    }
    if file_name.starts_with(query) {
        score += 800;
    }
    if let Some(pos) = file_name.find(query) {
        score += 600 - (pos as i64).min(200);
    }
    if normalized.starts_with(query) {
        score += 400;
    }
    if let Some(pos) = normalized.find(query) {
        score += 280 - (pos as i64).min(200);
    }

    Some(score)
}

pub(super) fn is_file_search_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | ':' | '#')
}

pub(super) fn trailing_token(text: &str) -> &str {
    let mut token_start = 0usize;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            token_start = i + c.len_utf8();
        }
    }
    &text[token_start..]
}

pub(super) fn active_file_search_query(text: &str) -> Option<&str> {
    let token = trailing_token(text);
    let at_index = token.rfind('@')?;
    let query = &token[at_index + 1..];
    query.chars().all(is_file_search_char).then_some(query)
}

pub(super) fn replace_or_append_file_mention(text: &str, mention_with_at: &str) -> String {
    let token = trailing_token(text);
    let token_start = text.len().saturating_sub(token.len());
    let mention_with_space = format!("{} ", mention_with_at);

    if let Some(at_index) = token.rfind('@') {
        let query = &token[at_index + 1..];
        if query.chars().all(is_file_search_char) {
            let replace_start = token_start + at_index;
            let mut new_text = text.to_string();
            new_text.replace_range(replace_start.., &mention_with_space);
            return new_text;
        }
    }

    if token.starts_with('@') && token[1..].chars().all(is_file_search_char) {
        let mut new_text = text.to_string();
        new_text.replace_range(token_start.., &mention_with_space);
        return new_text;
    }

    let mut new_text = text.to_string();
    if !new_text.is_empty() && !new_text.ends_with(' ') {
        new_text.push(' ');
    }
    new_text.push_str(&mention_with_space);
    new_text
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
