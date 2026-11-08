//! 处理 Git diff 面板消息，负责差异加载、刷新和选择状态更新。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

#[cfg(not(target_arch = "wasm32"))]
use crate::app::git::{
    git_discard_file, git_discard_hunk, git_revert_line_delete, git_revert_line_restore,
};
use crate::app::{
    App, Message,
    state::{
        GitDiffCommentDraft, GitDiffContextMenuState, GitDiffFileMenuState, GitDiffLineRange,
        GitDiffSelectedLine,
    },
};
use iced::Task;
use iced::widget::text_editor;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use super::shared::{changed_diff_line_sets, git_repo_path_for_app, new_position_for_old_line};
use super::shared::{
    clear_stage_selection_for_lines, diff_context_target_range, diff_context_target_stage_lines,
    dismiss_preview_transient_ui, extend_stage_selection_for_lines, load_missing_diff_contents,
    normalize_range, refresh_git_panel_data_task, selected_lines_from_range,
};
use super::{ExpandDirection, GitMessage};

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::ToggleDiffHighlight(enabled) => {
            app.show_git_diff_highlight = enabled;
            app.save_desktop_preference("show_git_diff_highlight", serde_json::Value::Bool(enabled))
        }
        GitMessage::ToggleDiffLineSelection(file, line, is_old, text) => {
            app.git_diff_selected_range = None;
            if let Some(pos) = app.git_diff_selected_lines.iter().position(|selected| {
                selected.file == file && selected.line == line && selected.is_old == is_old
            }) {
                app.git_diff_selected_lines.remove(pos);
            } else {
                app.git_diff_selected_lines.push(GitDiffSelectedLine { file, line, is_old, text });
            }
            Task::none()
        }
        GitMessage::DiffDragSelectStart(file, line, is_old, text) => {
            app.git_diff_comment_draft = None;
            app.git_diff_context_menu = None;
            app.git_diff_selected_range = None;
            app.git_diff_dragging = true;
            app.git_diff_drag_start_text = Some(text);
            app.git_diff_drag_range =
                Some(GitDiffLineRange { file, start: line, end: line, is_old });
            Task::none()
        }
        GitMessage::DiffDragSelectHover(file, line, is_old) => {
            if app.git_diff_dragging
                && let Some(range) = app.git_diff_drag_range.as_mut()
                && range.file == file
                && range.is_old == is_old
            {
                range.end = line;
            }
            Task::none()
        }
        GitMessage::DiffDragSelectEnd => {
            app.git_diff_dragging = false;
            let start_text = app.git_diff_drag_start_text.take();
            let Some(range) = app.git_diff_drag_range.take() else {
                return Task::none();
            };
            let range = normalize_range(range);

            let now = web_time::Instant::now();
            if range.start == range.end {
                let is_double_click =
                    app.git_diff_last_click.as_ref().is_some_and(|(file, line, is_old, when)| {
                        file == &range.file
                            && *line == range.start
                            && *is_old == range.is_old
                            && now.duration_since(*when) <= Duration::from_millis(350)
                    });

                if is_double_click {
                    app.git_diff_last_click = None;
                    app.git_diff_selected_range = Some(range.clone());
                    app.git_diff_comment_draft =
                        Some(GitDiffCommentDraft { range, editor: text_editor::Content::new() });
                } else {
                    app.git_diff_last_click =
                        Some((range.file.clone(), range.start, range.is_old, now));
                    app.git_diff_selected_range = None;
                    let text = start_text.unwrap_or_default();
                    if let Some(pos) = app.git_diff_selected_lines.iter().position(|selected| {
                        selected.file == range.file
                            && selected.line == range.start
                            && selected.is_old == range.is_old
                    }) {
                        app.git_diff_selected_lines.remove(pos);
                    } else {
                        app.git_diff_selected_lines.push(GitDiffSelectedLine {
                            file: range.file.clone(),
                            line: range.start,
                            is_old: range.is_old,
                            text,
                        });
                    }
                }
                return Task::none();
            }

            app.git_diff_last_click = None;
            app.git_diff_selected_range = Some(range.clone());
            app.git_diff_selected_lines = selected_lines_from_range(app, &range);
            app.git_diff_comment_draft = None;
            Task::none()
        }
        GitMessage::OpenDiffContextMenu { file, line, is_old, text, x, y } => {
            dismiss_preview_transient_ui(app);

            let in_selected_range = app.git_diff_selected_range.as_ref().is_some_and(|range| {
                range.file == file
                    && range.is_old == is_old
                    && line >= range.start.min(range.end)
                    && line <= range.start.max(range.end)
            });
            let in_selected_lines = app.git_diff_selected_lines.iter().any(|selected| {
                selected.file == file && selected.line == line && selected.is_old == is_old
            });

            if !in_selected_range
                && !in_selected_lines
                && app.git_diff_selected_lines.is_empty()
                && app.git_diff_selected_range.is_none()
            {
                app.git_diff_selected_lines.clear();
                app.git_diff_selected_lines.push(GitDiffSelectedLine {
                    file: file.clone(),
                    line,
                    is_old,
                    text,
                });
                app.git_diff_selected_range =
                    Some(GitDiffLineRange { file: file.clone(), start: line, end: line, is_old });
            }

            app.git_diff_context_menu = Some(GitDiffContextMenuState { file, line, is_old, x, y });
            Task::none()
        }
        GitMessage::CloseDiffContextMenu => {
            app.git_diff_context_menu = None;
            Task::none()
        }
        GitMessage::OpenDiffFileMenu(file) => {
            dismiss_preview_transient_ui(app);
            app.git_diff_context_menu = None;
            app.git_diff_file_menu = Some(GitDiffFileMenuState { file });
            Task::none()
        }
        GitMessage::CloseDiffFileMenu => {
            app.git_diff_file_menu = None;
            Task::none()
        }
        GitMessage::PreviewDiffFile(file) => {
            app.git_diff_file_menu = None;
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            if let Some(error) = crate::app::preview::preview_open_error_message(&full_path) {
                let fallback = error
                    .render(crate::app::i18n::I18n::new(
                        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .expect("preview error template");
                let task = app.show_error_toast(fallback);
                if let Some(toast) = &mut app.active_toast {
                    toast.localized_message = Some(error);
                }
                return task;
            }
            Task::done(Message::Preview(crate::app::message::PreviewMessage::Open(full_path)))
        }
        GitMessage::CopyDiffFile { file, deleted_content } => {
            app.git_diff_file_menu = None;
            if let Some(content) = deleted_content {
                return Task::done(Message::CopyCode(content));
            }
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            Task::done(Message::CopyFile(full_path))
        }
        GitMessage::RevertDiffFile(file) => {
            app.git_diff_file_menu = None;
            app.file_to_discard = Some(file);
            Task::none()
        }
        GitMessage::OpenDiffCommentDraft => {
            let Some(range) = diff_context_target_range(app) else {
                return Task::none();
            };
            app.git_diff_context_menu = None;
            app.git_diff_selected_range = Some(range.clone());
            app.git_diff_comment_draft =
                Some(GitDiffCommentDraft { range, editor: text_editor::Content::new() });
            Task::none()
        }
        GitMessage::SelectDiffContextStageLines => {
            let lines = diff_context_target_stage_lines(app);
            if lines.is_empty() {
                return Task::none();
            }
            extend_stage_selection_for_lines(app, &lines);
            app.git_diff_context_menu = None;
            Task::none()
        }
        GitMessage::ClearDiffContextStageLines => {
            let lines = diff_context_target_stage_lines(app);
            if lines.is_empty() {
                return Task::none();
            }
            clear_stage_selection_for_lines(app, &lines);
            app.git_diff_context_menu = None;
            Task::none()
        }
        GitMessage::DiffHoverEnter(file, line, is_old) => {
            app.git_diff_hovered_line = Some((file, line, is_old));
            Task::none()
        }
        GitMessage::DiffHoverExit(file, line, is_old) => {
            if let Some((hover_file, hover_line, hover_old)) = app.git_diff_hovered_line.as_ref()
                && hover_file == &file
                && *hover_line == line
                && *hover_old == is_old
            {
                app.git_diff_hovered_line = None;
            }
            Task::none()
        }
        GitMessage::DiffCommentEditorAction(action) => {
            if let Some(draft) = app.git_diff_comment_draft.as_mut() {
                draft.editor.perform(action);
            }
            Task::none()
        }
        GitMessage::DiffCommentCancel => {
            app.git_diff_comment_draft = None;
            app.git_diff_selected_range = None;
            Task::none()
        }
        GitMessage::DiffCommentSubmit => {
            let Some(draft) = app.git_diff_comment_draft.take() else {
                return Task::none();
            };
            let file = draft.range.file;
            let start = draft.range.start + 1;
            let end = draft.range.end + 1;
            let comment = draft.editor.text().trim().to_string();
            let line_ref =
                if start == end { start.to_string() } else { format!("{}-{}", start, end) };
            let out = if comment.is_empty() {
                format!("@{}:{} ", file, line_ref)
            } else {
                format!("@{}:{} #{} ", file, line_ref, comment)
            };
            app.git_diff_selected_range = None;
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(out)))
        }
        GitMessage::CopyDiffSelection => {
            app.git_diff_context_menu = None;
            let mut selected = app.git_diff_selected_lines.clone();
            selected.sort_by(|left, right| {
                left.file
                    .cmp(&right.file)
                    .then(left.is_old.cmp(&right.is_old))
                    .then(left.line.cmp(&right.line))
            });
            let content =
                selected.into_iter().map(|entry| entry.text).collect::<Vec<_>>().join("\n");
            if content.trim().is_empty() {
                Task::none()
            } else {
                Task::done(Message::CopyCode(content))
            }
        }
        GitMessage::DiscardDiffSelection => {
            app.git_diff_context_menu = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                let Some(repo_path) = git_repo_path_for_app(app) else {
                    return Task::none();
                };

                let mut selected = app.git_diff_selected_lines.clone();
                selected.sort_by(|left, right| {
                    left.file
                        .cmp(&right.file)
                        .then(left.is_old.cmp(&right.is_old))
                        .then(left.line.cmp(&right.line))
                });
                if selected.is_empty() {
                    return Task::none();
                }

                let mut refreshed = false;
                let mut changed_line_cache = std::collections::HashMap::new();
                for entry in selected.into_iter().rev() {
                    let (old_changed, new_changed) =
                        changed_line_cache.entry(entry.file.clone()).or_insert_with(|| {
                            changed_diff_line_sets(app, &entry.file).unwrap_or_default()
                        });

                    let result = if entry.is_old && old_changed.contains(&entry.line) {
                        let insert_idx = new_position_for_old_line(app, &entry.file, entry.line)
                            .unwrap_or(usize::MAX);
                        git_revert_line_restore(&repo_path, &entry.file, insert_idx, entry.line)
                    } else if !entry.is_old && new_changed.contains(&entry.line) {
                        git_revert_line_delete(&repo_path, &entry.file, entry.line)
                    } else {
                        continue;
                    };
                    if result.is_ok() {
                        refreshed = true;
                    }
                }

                app.git_diff_selected_lines.clear();
                app.git_diff_selected_range = None;
                if refreshed {
                    return refresh_git_panel_data_task();
                }
            }
            Task::none()
        }
        GitMessage::InsertDiffSelectionToChat => {
            app.git_diff_context_menu = None;
            let mut selected = app.git_diff_selected_lines.clone();
            selected.sort_by(|left, right| {
                left.file
                    .cmp(&right.file)
                    .then(left.is_old.cmp(&right.is_old))
                    .then(left.line.cmp(&right.line))
            });
            let text = selected.into_iter().map(|entry| entry.text).collect::<Vec<_>>().join("\n");
            if text.trim().is_empty() {
                Task::none()
            } else {
                Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(text)))
            }
        }
        GitMessage::InsertDiffSelectionComment => {
            app.git_diff_context_menu = None;
            let mut selected = app.git_diff_selected_lines.clone();
            selected.sort_by(|left, right| {
                left.file
                    .cmp(&right.file)
                    .then(left.is_old.cmp(&right.is_old))
                    .then(left.line.cmp(&right.line))
            });
            if selected.is_empty() {
                return Task::none();
            }

            let mut out = String::from("Git Diff 评论\n");
            for file in selected
                .iter()
                .map(|entry| entry.file.as_str())
                .collect::<std::collections::BTreeSet<_>>()
            {
                out.push_str(&format!("文件: {}\n", file));
                let old_lines = selected
                    .iter()
                    .filter(|entry| entry.file == file && entry.is_old)
                    .map(|entry| entry.line)
                    .collect::<Vec<_>>();
                let new_lines = selected
                    .iter()
                    .filter(|entry| entry.file == file && !entry.is_old)
                    .map(|entry| entry.line)
                    .collect::<Vec<_>>();
                if !old_lines.is_empty() {
                    out.push_str(&format!("旧行: {:?}\n", old_lines));
                }
                if !new_lines.is_empty() {
                    out.push_str(&format!("新行: {:?}\n", new_lines));
                }
                out.push_str("```text\n");
                for entry in selected.iter().filter(|entry| entry.file == file) {
                    out.push_str(&entry.text);
                    out.push('\n');
                }
                out.push_str("```\n");
            }
            out.push_str("评论: \n");

            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(out)))
        }
        GitMessage::ConfirmDiscardFile(file) => {
            app.git_diff_file_menu = None;
            app.file_to_discard = Some(file);
            Task::none()
        }
        GitMessage::CancelDiscardFile => {
            app.git_diff_file_menu = None;
            app.file_to_discard = None;
            Task::none()
        }
        GitMessage::DiscardFile(_file) => {
            app.git_diff_file_menu = None;
            app.file_to_discard = None;
            if let Some(_path) = &app.project_path {
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(error) = git_discard_file(_path, &_file) {
                    let _ = error;
                }
            }
            refresh_git_panel_data_task()
        }
        GitMessage::DiscardHunk(_file, _idx) => {
            if let Some(_path) = &app.project_path {
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(error) = git_discard_hunk(_path, &_file, _idx, app.terminal.shell) {
                    let _ = error;
                }
            }
            refresh_git_panel_data_task()
        }
        GitMessage::RevertLineDelete(_file, _line_idx) => {
            if let Some(_path) = &app.project_path {
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(error) = git_revert_line_delete(_path, &_file, _line_idx) {
                    let _ = error;
                }
            }
            refresh_git_panel_data_task()
        }
        GitMessage::RevertLineRestore(_file, _new_idx, _old_idx) => {
            if let Some(_path) = &app.project_path {
                #[cfg(not(target_arch = "wasm32"))]
                if let Err(error) = git_revert_line_restore(_path, &_file, _new_idx, _old_idx) {
                    let _ = error;
                }
            }
            refresh_git_panel_data_task()
        }
        GitMessage::ToggleFullscreen => {
            app.git_diff_fullscreen = !app.git_diff_fullscreen;
            app.git_diff_half_fullscreen = false;
            app.fullscreen_layout_settling = true;
            if app.git_diff_fullscreen {
                app.chat_panel_fullscreen = false;
                app.chat_panel_half_fullscreen = false;
                app.show_diff = true;
            }
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(crate::app::message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        GitMessage::ToggleHalfFullscreen => {
            app.git_diff_half_fullscreen = !app.git_diff_half_fullscreen;
            app.git_diff_fullscreen = false;
            app.fullscreen_layout_settling = true;
            if app.git_diff_half_fullscreen {
                app.chat_panel_fullscreen = false;
                app.chat_panel_half_fullscreen = false;
                app.show_diff = true;
            }
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(crate::app::message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        GitMessage::ToggleExpandHunk(file, idx) => {
            if let Some(pos) =
                app.expanded_hunks.iter().position(|(expanded_file, expanded_idx)| {
                    expanded_file == &file && *expanded_idx == idx
                })
            {
                app.expanded_hunks.remove(pos);
            } else {
                app.expanded_hunks.push((file, idx));
            }
            Task::none()
        }
        GitMessage::ExpandContext(file, gap_idx, direction) => {
            let entry = app.context_expansions.entry((file, gap_idx)).or_insert((0, 0));
            match direction {
                ExpandDirection::Down => entry.0 += 20,
                ExpandDirection::Up => entry.1 += 20,
                ExpandDirection::All => {
                    entry.0 = usize::MAX / 2;
                    entry.1 = usize::MAX / 2;
                }
            }
            Task::none()
        }
        GitMessage::ToggleExpandFile(name) => {
            if app.is_diff_file_expanded(&name) {
                app.clear_expanded_files();
                Task::none()
            } else {
                app.set_single_expanded_file(name.clone());
                load_missing_diff_contents(app, std::iter::once(name))
            }
        }
        GitMessage::FocusFile(name) => {
            dismiss_preview_transient_ui(app);
            app.git_focused_file = Some(name.clone());
            app.ensure_diff_file_expanded(name.clone());
            app.active_preview_path = None;
            app.show_diff = true;
            app.chat_text_diff = None;
            load_missing_diff_contents(app, std::iter::once(name))
        }
        GitMessage::DiffScrollChanged { offset_y, viewport_h } => {
            app.git_diff_scroll_offset_y = offset_y.clamp(0.0, 1.0);
            app.git_diff_scroll_viewport_h = viewport_h.max(0.0);
            Task::none()
        }
        GitMessage::DiffThemeSelected(theme) => {
            app.diff_theme = theme;
            Task::none()
        }
        _ => unreachable!("unexpected diff git message"),
    }
}
