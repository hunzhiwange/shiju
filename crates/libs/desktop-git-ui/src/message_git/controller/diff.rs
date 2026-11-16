//! 本领域完整消息分支；宿主只执行返回的跨域和系统效果。
use super::Context;
use super::Effect;
use super::GitMessage;
use super::State;
use super::Task;
use super::dismiss;
use super::load_missing;
use super::refresh_task;
use crate::message_git::messages::ExpandDirection;
pub fn update(
    app: &mut State,
    ctx: &Context,
    effects: &mut Vec<Effect>,
    message: GitMessage,
) -> Task<GitMessage> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::ToggleDiffHighlight(enabled) => {
            app.show_git_diff_highlight = enabled;
            {
                effects.push(Effect::PersistHighlight(enabled));
                Task::none()
            }
        }
        GitMessage::ToggleDiffLineSelection(file, line, is_old, text) => {
            crate::message_git::diff::toggle_diff_line_selection(
                &mut app.git_diff_selected_range,
                &mut app.git_diff_selected_lines,
                file,
                line,
                is_old,
                text,
            );
            Task::none()
        }
        GitMessage::DiffDragSelectStart(file, line, is_old, text) => {
            crate::message_git::diff::diff_drag_select_start(
                &mut app.git_diff_comment_draft,
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_dragging,
                &mut app.git_diff_drag_start_text,
                &mut app.git_diff_drag_range,
                file,
                line,
                is_old,
                text,
            );
            Task::none()
        }
        GitMessage::DiffDragSelectHover(file, line, is_old) => {
            crate::message_git::diff::diff_drag_select_hover(
                &mut app.git_diff_dragging,
                &mut app.git_diff_drag_range,
                file,
                line,
                is_old,
            );
            Task::none()
        }
        GitMessage::DiffDragSelectEnd => {
            let repo_path = ctx.repo_path.clone();
            let effect = crate::message_git::diff::diff_drag_select_end(
                &mut app.git_diff_dragging,
                &mut app.git_diff_drag_start_text,
                &mut app.git_diff_drag_range,
                &mut app.git_diff_last_click,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_comment_draft,
                &mut app.git_diff_selected_lines,
                repo_path,
            );
            apply_effect(effects, effect)
        }
        GitMessage::OpenDiffContextMenu { file, line, is_old, text, x, y } => {
            dismiss(app, effects);
            let effect = crate::message_git::diff::open_diff_context_menu(
                &mut app.git_diff_selected_range,
                &mut app.git_diff_selected_lines,
                &mut app.git_diff_context_menu,
                file,
                line,
                is_old,
                text,
                x,
                y,
            );
            apply_effect(effects, effect)
        }
        GitMessage::CloseDiffContextMenu => {
            crate::message_git::diff::close_diff_context_menu(&mut app.git_diff_context_menu);
            Task::none()
        }
        GitMessage::OpenDiffFileMenu(file) => {
            dismiss(app, effects);
            let effect = crate::message_git::diff::open_diff_file_menu(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_file_menu,
                file,
            );
            apply_effect(effects, effect)
        }
        GitMessage::CloseDiffFileMenu => {
            crate::message_git::diff::close_diff_file_menu(&mut app.git_diff_file_menu);
            Task::none()
        }
        GitMessage::PreviewDiffFile(file) => {
            app.git_diff_file_menu = None;
            let repo_path = ctx.repo_path.clone();
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            effects.push(Effect::Preview(full_path));
            Task::none()
        }
        GitMessage::CopyDiffFile { file, deleted_content } => {
            app.git_diff_file_menu = None;
            if let Some(content) = deleted_content {
                effects.push(Effect::CopyCode(content));
                return Task::none();
            }
            let repo_path = ctx.repo_path.clone();
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            {
                effects.push(Effect::CopyFile(full_path));
                Task::none()
            }
        }
        GitMessage::RevertDiffFile(file) => {
            crate::message_git::diff::revert_diff_file(
                &mut app.git_diff_file_menu,
                &mut app.file_to_discard,
                file,
            );
            Task::none()
        }
        GitMessage::OpenDiffCommentDraft => {
            let effect = crate::message_git::diff::open_diff_comment_draft(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_selected_lines,
                &mut app.git_diff_comment_draft,
            );
            apply_effect(effects, effect)
        }
        GitMessage::SelectDiffContextStageLines => {
            let effect = crate::message_git::diff::select_diff_context_stage_lines(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_selected_lines,
                &mut app.staged_old_lines_selected,
                &mut app.staged_lines_selected,
            );
            apply_effect(effects, effect)
        }
        GitMessage::ClearDiffContextStageLines => {
            let effect = crate::message_git::diff::clear_diff_context_stage_lines(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_selected_lines,
                &mut app.staged_old_lines_selected,
                &mut app.staged_lines_selected,
            );
            apply_effect(effects, effect)
        }
        GitMessage::DiffHoverEnter(file, line, is_old) => {
            crate::message_git::diff::diff_hover_enter(
                &mut app.git_diff_hovered_line,
                file,
                line,
                is_old,
            );
            Task::none()
        }
        GitMessage::DiffHoverExit(file, line, is_old) => {
            crate::message_git::diff::diff_hover_exit(
                &mut app.git_diff_hovered_line,
                file,
                line,
                is_old,
            );
            Task::none()
        }
        GitMessage::DiffCommentEditorAction(action) => {
            crate::message_git::diff::diff_comment_editor_action(
                &mut app.git_diff_comment_draft,
                action,
            );
            Task::none()
        }
        GitMessage::DiffCommentCancel => {
            crate::message_git::diff::diff_comment_cancel(
                &mut app.git_diff_comment_draft,
                &mut app.git_diff_selected_range,
            );
            Task::none()
        }
        GitMessage::DiffCommentSubmit => {
            let effect = crate::message_git::diff::diff_comment_submit(
                &mut app.git_diff_comment_draft,
                &mut app.git_diff_selected_range,
            );
            apply_effect(effects, effect)
        }
        GitMessage::CopyDiffSelection => {
            let effect = crate::message_git::diff::copy_diff_selection(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_lines,
            );
            apply_effect(effects, effect)
        }
        GitMessage::DiscardDiffSelection => {
            let repo_path = ctx.repo_path.clone();
            let effect = crate::message_git::diff::discard_diff_selection(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_lines,
                &mut app.git_diff_selected_range,
                repo_path,
            );
            apply_effect(effects, effect)
        }
        GitMessage::InsertDiffSelectionToChat => {
            let effect = crate::message_git::diff::insert_diff_selection_to_chat(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_lines,
            );
            apply_effect(effects, effect)
        }
        GitMessage::InsertDiffSelectionComment => {
            let effect = crate::message_git::diff::insert_diff_selection_comment(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_selected_lines,
            );
            apply_effect(effects, effect)
        }
        GitMessage::ConfirmDiscardFile(file) => {
            crate::message_git::diff::confirm_discard_file(
                &mut app.git_diff_file_menu,
                &mut app.file_to_discard,
                file,
            );
            Task::none()
        }
        GitMessage::CancelDiscardFile => {
            crate::message_git::diff::cancel_discard_file(
                &mut app.git_diff_file_menu,
                &mut app.file_to_discard,
            );
            Task::none()
        }
        GitMessage::DiscardFile(_file) => {
            crate::message_git::diff::cancel_discard_file(
                &mut app.git_diff_file_menu,
                &mut app.file_to_discard,
            );
            effects.push(Effect::Discard(crate::message_git::diff::DiscardAction::File(_file)));
            refresh_task()
        }
        GitMessage::DiscardHunk(_file, _idx) => {
            effects
                .push(Effect::Discard(crate::message_git::diff::DiscardAction::Hunk(_file, _idx)));
            refresh_task()
        }
        GitMessage::RevertLineDelete(_file, _line_idx) => {
            effects.push(Effect::Discard(crate::message_git::diff::DiscardAction::DeleteLine(
                _file, _line_idx,
            )));
            refresh_task()
        }
        GitMessage::RevertLineRestore(_file, _new_idx, _old_idx) => {
            effects.push(Effect::Discard(crate::message_git::diff::DiscardAction::RestoreLine(
                _file, _new_idx, _old_idx,
            )));
            refresh_task()
        }
        GitMessage::ToggleFullscreen => {
            let active = crate::message_git::diff::toggle_fullscreen(
                &mut app.git_diff_fullscreen,
                &mut app.git_diff_half_fullscreen,
                false,
            );
            effects.push(Effect::Fullscreen(active));
            Task::none()
        }
        GitMessage::ToggleHalfFullscreen => {
            let active = crate::message_git::diff::toggle_fullscreen(
                &mut app.git_diff_fullscreen,
                &mut app.git_diff_half_fullscreen,
                true,
            );
            effects.push(Effect::Fullscreen(active));
            Task::none()
        }
        GitMessage::ToggleExpandHunk(file, idx) => {
            crate::message_git::diff::toggle_expand_hunk(&mut app.expanded_hunks, file, idx);
            Task::none()
        }
        GitMessage::ExpandContext(file, gap_idx, direction) => {
            let direction = match direction {
                ExpandDirection::Up => crate::message_git::diff::ExpandDirection::Up,
                ExpandDirection::Down => crate::message_git::diff::ExpandDirection::Down,
                ExpandDirection::All => crate::message_git::diff::ExpandDirection::All,
            };
            crate::message_git::diff::expand_context(
                &mut app.context_expansions,
                file,
                gap_idx,
                direction,
            );
            Task::none()
        }
        GitMessage::ToggleExpandFile(name) => {
            if crate::message_git::diff::toggle_expand_file(
                &mut app.expanded_files,
                &mut app.expanded_files_set,
                name.clone(),
            ) {
                load_missing(app, ctx.repo_path.clone(), effects, std::iter::once(name))
            } else {
                Task::none()
            }
        }
        GitMessage::FocusFile(name) => {
            dismiss(app, effects);
            app.git_focused_file = Some(name.clone());
            crate::message_git::diff::ensure_diff_file_expanded(
                &mut app.expanded_files_set,
                &mut app.expanded_files,
                name.clone(),
            );
            effects.push(Effect::ClearPreview);
            effects.push(Effect::ShowDiff { changes: false });
            app.chat_text_diff = None;
            load_missing(app, ctx.repo_path.clone(), effects, std::iter::once(name))
        }
        GitMessage::DiffScrollChanged { offset_y, viewport_h } => {
            crate::message_git::diff::scroll_changed(
                &mut app.git_diff_scroll_offset_y,
                &mut app.git_diff_scroll_viewport_h,
                offset_y,
                viewport_h,
            );
            Task::none()
        }
        GitMessage::DiffThemeSelected(theme) => {
            crate::message_git::diff::diff_theme_selected(&mut app.diff_theme, theme);
            Task::none()
        }
        _ => unreachable!("unexpected diff git message"),
    }
}

fn apply_effect(
    effects: &mut Vec<Effect>,
    effect: crate::message_git::diff::Effect,
) -> Task<GitMessage> {
    match effect {
        crate::message_git::diff::Effect::None => Task::none(),
        crate::message_git::diff::Effect::CopyCode(text) => {
            effects.push(Effect::CopyCode(text));
            Task::none()
        }
        crate::message_git::diff::Effect::AppendChat(text) => {
            effects.push(Effect::AppendChat(text));
            Task::none()
        }
        crate::message_git::diff::Effect::Refresh => refresh_task(),
    }
}
