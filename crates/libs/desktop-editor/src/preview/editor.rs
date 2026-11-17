//! 预览编辑器局部更新与效果。
use super::selection::{editor_message_may_change_content, update_context_target_from_editor_event};

use super::message::PreviewMessage;
use config_ui::PreviewAutoSaveMode;
use iced::Task;
#[cfg(not(target_arch = "wasm32"))] use super::lsp;
#[cfg(not(target_arch = "wasm32"))] use super::lsp::Notifications;
#[cfg(not(target_arch = "wasm32"))] use std::time::Instant;
pub enum Effect {
    Preview(PreviewMessage),
    Editor(crate::message::editor::EditorMessage),
    AutoSave { path: String, revision: u64 },
}

pub fn update(
    active_preview_path: &mut Option<String>,
    cursor_position: &mut iced::Point,
    editor_popover: &mut Option<crate::components::preview_panel::view::options::EditorPopover>,
    #[cfg(not(target_arch = "wasm32"))] lsp_applying_completion: &mut bool,
    #[cfg(not(target_arch = "wasm32"))] lsp_disabled: &mut bool,
    #[cfg(not(target_arch = "wasm32"))] lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    #[cfg(not(target_arch = "wasm32"))] lsp_hover_hide_deadline: &mut Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))] lsp_hover_pending: &mut Option<super::LspHoverPending>,
    #[cfg(not(target_arch = "wasm32"))] lsp_manager: &mut Option<desktop_lsp::LspServiceManager>,
    #[cfg(not(target_arch = "wasm32"))] lsp_overlay: &mut iced_code_editor::LspOverlayState,
    #[cfg(not(target_arch = "wasm32"))] lsp_overlay_path: &mut Option<String>,
    #[cfg(not(target_arch = "wasm32"))] lsp_progress: &mut std::collections::HashMap<String, std::collections::HashMap<String, super::LspProgress>>,
    #[cfg(not(target_arch = "wasm32"))] lsp_status: &mut Option<String>,
    #[cfg(not(target_arch = "wasm32"))] lsp_status_message: &mut Option<(String, desktop_i18n::format::Message)>,
    #[cfg(not(target_arch = "wasm32"))] notifications: &mut Notifications,
    preview_auto_save_mode: &mut config_ui::PreviewAutoSaveMode,
    preview_context_menu_pos: &mut Option<(f32, f32)>,
    preview_context_target: &mut Option<(String, usize, usize, usize, usize)>,
    #[cfg(not(target_arch = "wasm32"))] preview_nav_popup: &mut Option<(String, f32, f32, Vec<(String, bool)>)>,
    preview_tabs: &mut Vec<super::PreviewTab>,
    #[cfg(not(target_arch = "wasm32"))] project_path: &mut Option<String>,
    show_preview_context_menu: &mut bool,
    show_preview_fullscreen_overlay: &mut bool,
    focus_preview: &mut bool,
    #[cfg(not(target_arch = "wasm32"))] persist_lsp: &mut Option<bool>,
    message: PreviewMessage,
) -> Task<Effect> {
    match message {
        PreviewMessage::EditorEvent(evt) => {
            let from_actions =
                (*editor_popover) == Some(crate::components::preview_panel::view::options::EditorPopover::Actions);
            if from_actions || matches!(evt, iced_code_editor::Message::OpenCommandPalette) {
                (*editor_popover) = None;
            }
            if active_preview_path.as_ref().is_some_and(|path| {
                preview_tabs.iter().any(|tab| tab.path == *path && tab.read_only)
            }) && (editor_message_may_change_content(&evt)
                || matches!(evt, iced_code_editor::Message::RevealInFileManager))
            {
                return Task::none();
            }
            // 更新上下文选择目标（用于右键菜单操作选中文本）
            update_context_target_from_editor_event(active_preview_path, preview_tabs, preview_context_target, &evt);

            // ===== LSP 代码补全交互处理（非 WASM 平台） =====
            #[cfg(not(target_arch = "wasm32"))]
            {
                if matches!(&evt, iced_code_editor::Message::ContextMenuRequested(_)) {
                    lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                    lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, true);
                }

                // 关闭搜索框时，如果补全列表可见，则清除补全
                if matches!(&evt, iced_code_editor::Message::CloseSearch)
                    && lsp_overlay.completion_visible
                {
                    lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
                    return Task::none();
                }

                // 当补全列表可见且未被抑制时，处理导航键
                if lsp_overlay.completion_visible
                    && !lsp_overlay.completion_suppressed
                    && !lsp_overlay.completion_items.is_empty()
                {
                    match &evt {
                        iced_code_editor::Message::ArrowKey(direction, false) => {
                            use iced_code_editor::ArrowDirection;
                            match direction {
                                // 上下箭头键导航补全列表
                                ArrowDirection::Up => {
                                    return Task::done(Effect::Preview(
                                        PreviewMessage::LspCompletionNavigateUp,
                                    ));
                                }
                                ArrowDirection::Down => {
                                    return Task::done(Effect::Preview(
                                        PreviewMessage::LspCompletionNavigateDown,
                                    ));
                                }
                                // 左右箭头键关闭补全列表
                                ArrowDirection::Left | ArrowDirection::Right => {
                                    lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
                                }
                            }
                        }
                        // 回车键确认选择补全项
                        iced_code_editor::Message::Enter => {
                            return Task::done(Effect::Preview(
                                PreviewMessage::LspCompletionConfirm,
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // 将事件传递给编辑器处理
            *focus_preview = true;

            if let Some(path) = active_preview_path.as_deref()
                && let Some(tab) = preview_tabs.iter_mut().find(|t| t.path == path)
            {
                if from_actions {
                    tab.editor.inner.request_focus();
                    let _ = tab.editor.inner.update(&iced_code_editor::Message::CanvasFocusGained);
                }
                let was_dirty = tab.is_dirty;
                let may_change_content = editor_message_may_change_content(&evt);
                let was_modified = may_change_content && tab.editor.inner.is_modified();
                let task = tab
                    .editor
                    .inner
                    .update(&evt)
                    .map(|e| Effect::Preview(PreviewMessage::EditorEvent(e)));
                let edit_attempted = if may_change_content {
                    let is_modified = tab.editor.inner.is_modified();
                    tab.is_dirty = is_modified;
                    was_modified || is_modified
                } else {
                    tab.is_dirty = was_dirty;
                    false
                };
                if edit_attempted {
                    tab.auto_save_revision = tab.auto_save_revision.saturating_add(1);
                }
                let auto_save_task = if edit_attempted
                    && tab.is_dirty
                    && matches!(
                        *preview_auto_save_mode,
                        PreviewAutoSaveMode::AfterDelay | PreviewAutoSaveMode::OnFocusChange
                    ) {
                    let revision = tab.auto_save_revision;
                    let auto_save_path = path.to_string();
                    Some(Task::done(Effect::AutoSave { path: auto_save_path, revision }))
                } else {
                    None
                };

                // ===== LSP 高级功能处理（非 WASM 平台） =====
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // 鼠标悬停：触发 LSP 类型/文档提示
                    if let iced_code_editor::Message::MouseHover(point) = &evt {
                        // 仅在上下文菜单和导航弹窗都未显示时处理
                        if !(*show_preview_context_menu) && preview_nav_popup.is_none() {
                            tracing::debug!("[LSP HOVER] MouseHover at point={:?}", point);
                            lsp::handle_lsp_hover_from_mouse(active_preview_path, lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path, preview_nav_popup, preview_tabs, show_preview_context_menu, *point);
                        }
                    }

                    // 鼠标释放：设置悬停提示的自动隐藏倒计时
                    if let iced_code_editor::Message::MouseRelease = &evt
                        && lsp_overlay.hover_visible
                    {
                        // 禁用交互模式，400ms 后自动隐藏
                        lsp_overlay.hover_interactive = false;
                        (*lsp_hover_hide_deadline) =
                            Some(std::time::Instant::now() + std::time::Duration::from_millis(400));
                    }

                    // Ctrl+点击：跳转到定义
                    if let iced_code_editor::Message::JumpClick(point) = &evt {
                        // 如果上下文菜单或导航弹窗已打开，不处理跳转
                        if (*show_preview_context_menu) || preview_nav_popup.is_some() {
                            return task;
                        }
                        // 请求 LSP 定义跳转
                        if let Some(path) = active_preview_path.clone()
                            && let Some(tab) = (*preview_tabs)
                                .iter_mut()
                                .find(|t| t.path.as_str() == path.as_str())
                        {
                            tab.editor.lsp_request_definition_at(*point);
                        }
                    }

                    // 字符输入：更新补全过滤或清除补全
                    if let iced_code_editor::Message::CharacterInput(ch) = &evt
                        && !(*lsp_applying_completion)
                    {
                        // 非字母数字下划线字符：清除补全列表
                        if !ch.is_alphanumeric() && *ch != '_' {
                            lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
                        } else {
                            // 字母数字下划线：取消抑制并更新补全过滤
                            lsp_overlay.completion_suppressed = false;

                            // 根据当前输入的单词前缀过滤补全列表
                            if !lsp_overlay.all_completions.is_empty()
                                && let Some(path) = active_preview_path.as_ref()
                                && let Some(tab) = preview_tabs.iter().find(|t| &t.path == path)
                            {
                                let content = tab.editor.content();
                                let (line, col) = tab.editor.cursor_position();

                                // 提取当前行的内容
                                if let Some(line_content) = content.lines().nth(line) {
                                    // 向前扫描找到单词起始位置
                                    let word_start = {
                                        let chars: Vec<char> = line_content.chars().collect();
                                        let mut start = col;
                                        while start > 0 {
                                            let c = chars.get(start - 1).copied().unwrap_or(' ');
                                            if !c.is_alphanumeric() && c != '_' {
                                                break;
                                            }
                                            start -= 1;
                                        }
                                        start
                                    };

                                    // 设置过滤字符串并执行过滤
                                    lsp_overlay.completion_filter =
                                        line_content[word_start..col].to_string();
                                    lsp::filter_completions(lsp_overlay);
                                }
                            }
                        }
                    }

                    // 字母数字或下划线输入：触发 LSP 补全请求
                    match &evt {
                        iced_code_editor::Message::CharacterInput(ch)
                            if ch.is_alphanumeric() || *ch == '_' =>
                        {
                            if let Some(path) = active_preview_path.clone()
                                && let Some(tab) = (*preview_tabs)
                                    .iter_mut()
                                    .find(|t| t.path.as_str() == path.as_str())
                            {
                                tab.editor.lsp_request_completion();
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(auto_save_task) = auto_save_task {
                    return Task::batch(vec![task, auto_save_task]);
                }

                return task;
            }
            Task::none()
        }

        // ===== 上下文菜单消息处理 =====

        // 打开上下文菜单（右键菜单）
        PreviewMessage::ContextMenuOpenForActiveEditor(x, y) => {
            (*editor_popover) = None;
            if let Some(path) = active_preview_path.clone() {



                // 检查是否需要重置选择目标为全选（1,1 到 1,1 表示无选择）
                let should_reset_target = !matches!(
                    *preview_context_target,
                    Some((ref p, start_line, start_col, end_line, end_col))
                        if *p == path && (start_line, start_col, end_line, end_col) != (1, 1, 1, 1)
                );

                if should_reset_target {
                    (*preview_context_target) = Some((path, 1, 1, 1, 1));
                }

                // 清除 LSP 相关浮层（非 WASM 平台）
                #[cfg(not(target_arch = "wasm32"))]
                {
                    lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                    lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, true);
                }

                // 显示上下文菜单并记录位置
                let (menu_x, menu_y) =
                    if cursor_position.x != 0.0 || cursor_position.y != 0.0 {
                        (cursor_position.x, cursor_position.y)
                    } else {
                        (x, y)
                    };

                (*show_preview_context_menu) = true;
                (*preview_context_menu_pos) = Some((menu_x, menu_y));
            }
            Task::none()
        }

        // 复制操作
        PreviewMessage::ContextMenuCopy => {
            (*show_preview_context_menu) = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
            }
            Task::done(Effect::Editor(crate::message::editor::EditorMessage::Copy))
        }

        // 剪切操作
        PreviewMessage::ContextMenuCut => {
            (*show_preview_context_menu) = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
            }
            Task::done(Effect::Editor(crate::message::editor::EditorMessage::Cut))
        }

        // 粘贴操作
        PreviewMessage::ContextMenuPaste => {
            (*show_preview_context_menu) = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
            }
            Task::done(Effect::Editor(crate::message::editor::EditorMessage::Paste))
        }

        // 删除操作
        PreviewMessage::ContextMenuDelete => {
            (*show_preview_context_menu) = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
            }
            Task::done(Effect::Editor(crate::message::editor::EditorMessage::Delete))
        }

        // 关闭上下文菜单
        PreviewMessage::ContextMenuClose => {
            (*show_preview_context_menu) = false;
            (*preview_context_target) = None;
            (*preview_context_menu_pos) = None;
            Task::none()
        }

        PreviewMessage::FullscreenOverlayEntered => {
            (*show_preview_fullscreen_overlay) = true;
            Task::none()
        }

        PreviewMessage::FullscreenOverlayExited => {
            (*show_preview_fullscreen_overlay) = false;
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::EditorMouseEntered => {
            if lsp_overlay_path.as_ref() == active_preview_path.as_ref() {
                (*lsp_hover_hide_deadline) = None;
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::EditorMouseExited => {
            if lsp_overlay_path.as_ref() == active_preview_path.as_ref()
                && lsp_overlay.hover_visible
                && !lsp_overlay.hover_interactive
            {
                (*lsp_hover_hide_deadline) =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(500));
            }
            Task::none()
        }

        // ===== LSP 悬停提示交互（非 WASM 平台） =====
        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspHoverEntered => {
            // 鼠标进入悬停提示区域：保持显示并启用交互
            lsp_overlay.hover_interactive = true;
            (*lsp_hover_hide_deadline) = None;
            lsp::focus_hover_overlay()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspHoverExited => {
            // 鼠标离开悬停提示区域：设置 300ms 后自动隐藏
            lsp_overlay.hover_interactive = false;
            (*lsp_hover_hide_deadline) =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(300));
            Task::none()
        }

        // ===== LSP 代码补全交互（非 WASM 平台） =====
        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionClosed => {
            // 补全列表关闭
            lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionSelected(index) => {
            // 鼠标点击选择补全项
            (*lsp_applying_completion) = true;
            let completion = lsp_overlay.completion_items.get(index).cloned();
            if let Some(item) = completion {
                lsp::apply_completion(active_preview_path, preview_tabs, &item);
            }
            (*lsp_applying_completion) = false;
            lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, true);
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionNavigateUp => {
            // 向上导航补全列表
            if lsp_overlay.completion_visible && !lsp_overlay.completion_items.is_empty() {
                // 循环导航：到达顶部时跳转到底部
                if lsp_overlay.completion_selected > 0 {
                    lsp_overlay.completion_selected -= 1;
                } else {
                    lsp_overlay.completion_selected =
                        lsp_overlay.completion_items.len() - 1;
                }
                // 返回滚动任务以保持选中项可见
                return lsp::completion_scroll_task(lsp_overlay.completion_selected);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionNavigateDown => {
            // 向下导航补全列表
            if lsp_overlay.completion_visible && !lsp_overlay.completion_items.is_empty() {
                // 循环导航：到达底部时跳转到顶部
                lsp_overlay.completion_selected = (lsp_overlay.completion_selected + 1)
                    % lsp_overlay.completion_items.len();
                // 返回滚动任务以保持选中项可见
                return lsp::completion_scroll_task(lsp_overlay.completion_selected);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionConfirm => {
            // 确认选择补全项（回车键）
            if lsp_overlay.completion_visible {
                let selected = lsp_overlay.completion_selected;
                (*lsp_applying_completion) = true;
                let completion = lsp_overlay.completion_items.get(selected).cloned();
                if let Some(item) = completion {
                    lsp::apply_completion(active_preview_path, preview_tabs, &item);
                }
                (*lsp_applying_completion) = false;
                lsp::clear_lsp_completion(lsp_overlay, lsp_overlay_path, true);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspToggleDisabled(disabled) => {

            if disabled {
                lsp::disable_lsp(lsp_disabled, lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_manager, lsp_overlay, lsp_overlay_path, lsp_progress, lsp_status, lsp_status_message, preview_tabs);
            } else {
                lsp::enable_lsp(active_preview_path, lsp_disabled, lsp_manager, lsp_status, lsp_status_message, notifications, preview_tabs, project_path);
            }
            { *persist_lsp = Some(!disabled); Task::none() }
        }

        // 处理其他未实现的消息变体，或由其他机制处理的消息（如选择变更）
        _ => Task::none(),
    }
}
