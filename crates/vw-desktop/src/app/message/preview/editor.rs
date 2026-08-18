//! 预览编辑器消息处理模块
//!
//! 本模块负责处理预览编辑器相关的所有消息和交互事件，包括：
//! - 编辑器事件的分发和处理
//! - 鼠标交互（点击、拖拽、悬停）的位置计算
//! - LSP（语言服务器协议）功能的集成（代码补全、悬停提示、跳转定义）
//! - 右键上下文菜单的管理
//!
//! # 主要功能
//!
//! 1. **编辑器事件处理**：将用户的键盘、鼠标输入转换为编辑器操作
//! 2. **LSP 集成**：提供智能代码补全、类型提示、定义跳转等 IDE 功能
//! 3. **上下文菜单**：支持复制、粘贴、剪切、删除等常用编辑操作
//!
//! # 架构说明
//!
//! 模块采用消息驱动架构，通过 `PreviewMessage` 枚举接收各类事件，
//! 并返回相应的 `Task` 以驱动异步操作（如 LSP 请求）。

use super::PreviewMessage;
#[cfg(not(target_arch = "wasm32"))]
use super::lsp;
use crate::app::components::editor::{
    APP_CONTEXT_MENU_INSERT_SELECTION, APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS,
};
use crate::app::{App, FocusArea, Message, PreviewAutoSaveMode};
use iced::Task;
use unicode_width::UnicodeWidthChar;

fn editor_message_may_change_content(message: &iced_code_editor::Message) -> bool {
    matches!(
        message,
        iced_code_editor::Message::CharacterInput(_)
            | iced_code_editor::Message::Backspace
            | iced_code_editor::Message::Delete
            | iced_code_editor::Message::Enter
            | iced_code_editor::Message::Tab
            | iced_code_editor::Message::Paste(_)
            | iced_code_editor::Message::Cut
            | iced_code_editor::Message::DeleteSelection
            | iced_code_editor::Message::Undo
            | iced_code_editor::Message::Redo
            | iced_code_editor::Message::ReplaceNext
            | iced_code_editor::Message::ReplaceAll
            | iced_code_editor::Message::ImeCommit(_)
    )
}

/// 从屏幕坐标点计算编辑器中的光标位置（行号和列号）
///
/// 该函数将屏幕上的鼠标点击位置转换为编辑器缓冲区中的逻辑位置，
/// 需要考虑行号装饰（gutter）的宽度以及字符的实际显示宽度（如中文字符）。
///
/// # 参数
///
/// * `editor` - 代码编辑器组件的引用
/// * `point` - 屏幕上的坐标点（像素坐标）
///
/// # 返回值
///
/// * `Some((line, col))` - 成功计算出的位置，行号和列号均为 1 起始索引
/// * `None` - 点击位置在行号装饰区域或其他无效区域
///
/// # 示例
///
/// ```ignore
/// let editor = &tab.editor.inner;
/// let point = iced::Point { x: 100.0, y: 50.0 };
/// if let Some((line, col)) = cursor_from_point(editor, point) {
///     println!("点击位置：第 {} 行，第 {} 列", line, col);
/// }
/// ```
///
/// # 算法说明
///
/// 1. 计算并排除行号装饰（gutter）的宽度
/// 2. 根据行高计算点击的行索引
/// 3. 遍历该行的每个字符，累加显示宽度以找到精确的列位置
fn cursor_from_point(
    editor: &iced_code_editor::CodeEditor,
    point: iced::Point,
) -> Option<(usize, usize)> {
    // 如果启用了行号显示，行号装饰宽度为 45 像素，否则为 0
    let gutter_width = if editor.line_numbers_enabled() { 45.0 } else { 0.0 };

    // 点击在行号装饰区域，返回 None
    if point.x < gutter_width {
        return None;
    }

    // 根据行高计算可视行索引（Y 坐标除以行高）
    let visual_line_idx = (point.y / editor.line_height()) as usize;

    // 获取编辑器内容并按换行符分割为行数组
    let content = editor.content();
    let lines: Vec<&str> = content.split('\n').collect();

    // 空内容时返回 (1, 1)
    if lines.is_empty() {
        return Some((1, 1));
    }

    // 确保行索引不超过实际行数（边界保护）
    let line_idx = visual_line_idx.min(lines.len().saturating_sub(1));
    let line = lines[line_idx];

    // 计算文本区域内的 X 坐标（减去行号装饰宽度和 5 像素的左边距）
    let x_in_text = (point.x - gutter_width - 5.0).max(0.0);

    // 遍历该行字符，累加显示宽度以定位列位置
    let mut current_width = 0.0;
    let mut col_offset = 0usize;
    for c in line.chars() {
        // 获取字符显示宽度：宽字符（如中文）使用 full_char_width，普通字符使用 char_width
        let w = match c.width() {
            Some(w) if w > 1 => editor.full_char_width(),
            Some(_) => editor.char_width(),
            None => 0.0,
        };

        // 如果当前宽度加上半个字符宽度超过点击 X 坐标，则定位到该字符
        if current_width + w / 2.0 > x_in_text {
            break;
        }
        current_width += w;
        col_offset += 1;
    }

    // 返回 1 起始的行号和列号
    Some((line_idx + 1, col_offset + 1))
}

/// 规范化两个位置为起始位置和结束位置（确保 start <= end）
///
/// 比较两个光标位置（行，列），按顺序返回（较小位置，较大位置），
/// 用于确定文本选择的范围。
///
/// # 参数
///
/// * `a` - 第一个位置 (行, 列)
/// * `b` - 第二个位置 (行, 列)
///
/// # 返回值
///
/// 返回有序的位置对 `((start_line, start_col), (end_line, end_col))`
///
/// # 比较规则
///
/// - 先比较行号：行号小的为起始位置
/// - 行号相同时比较列号：列号小的为起始位置
///
/// # 示例
///
/// ```ignore
/// let pos1 = (5, 10);
/// let pos2 = (3, 20);
/// let ((start_line, start_col), (end_line, end_col)) = normalize_pos(pos1, pos2);
/// assert_eq!((start_line, start_col), (3, 20)); // pos2 在前
/// assert_eq!((end_line, end_col), (5, 10));
/// ```
fn normalize_pos(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) { (a, b) } else { (b, a) }
}

/// 从编辑器事件中更新上下文选择目标
///
/// 根据鼠标点击和拖拽事件，更新应用状态中的 `preview_context_target`，
/// 用于跟踪用户选择的文本范围（支持右键菜单操作选中文本）。
///
/// # 参数
///
/// * `app` - 应用状态的可变引用
/// * `evt` - 编辑器产生的事件消息
///
/// # 处理的事件类型
///
/// - `MouseClick`: 单击时设置光标位置为选择起点和终点（无范围选择）
/// - `MouseDrag`: 拖拽时更新选择终点，形成文本选择范围
///
/// # 选择范围格式
///
/// `preview_context_target` 存储为 `Some((path, start_line, start_col, end_line, end_col))`，
/// 其中路径用于验证选择是否仍然有效（防止跨文件选择混乱）。
fn update_context_target_from_editor_event(app: &mut App, evt: &iced_code_editor::Message) {
    // 获取当前活动的预览文件路径
    let Some(path) = app.active_preview_path.clone() else {
        return;
    };

    // 查找对应的预览标签页
    let Some(tab) = app.preview_tabs.iter().find(|t| t.path == path) else {
        return;
    };

    match evt {
        iced_code_editor::Message::MouseClick(point) => {
            // 鼠标单击：设置光标位置为单点选择（起点=终点）
            if let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) {
                app.preview_context_target = Some((path, line, col, line, col));
            }
        }
        iced_code_editor::Message::ContextMenuRequested(point) => {
            let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) else {
                return;
            };
            let clicked_inside_active_selection = matches!(
                app.preview_context_target,
                Some((ref target_path, start_line, start_col, end_line, end_col))
                    if *target_path == path
                        && (start_line, start_col) != (end_line, end_col)
                        && (start_line, start_col) <= (line, col)
                        && (line, col) <= (end_line, end_col)
            );
            if !clicked_inside_active_selection {
                app.preview_context_target = Some((path, line, col, line, col));
            }
        }
        iced_code_editor::Message::MouseDrag(point) => {
            // 鼠标拖拽：更新选择范围
            let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) else {
                return;
            };

            // 检查是否存在有效的选择起点，且路径匹配
            if let Some((target_path, start_line, start_col, _, _)) =
                app.preview_context_target.clone()
                && target_path == path
            {
                // 规范化选择范围（确保 start <= end）
                let ((sl, sc), (el, ec)) = normalize_pos((start_line, start_col), (line, col));
                app.preview_context_target = Some((path, sl, sc, el, ec));
            } else {
                // 无有效起点或路径不匹配，从当前位置开始新选择
                app.preview_context_target = Some((path, line, col, line, col));
            }
        }
        _ => {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    let path = std::path::Path::new(path);
    if !path.is_file() {
        return Err(format!("无法在文件管理器中显示：文件不存在 {}", path.display()));
    }

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg("-R").arg(path);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg("/select,").arg(path);
        command
    };

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = {
        let directory =
            path.parent().ok_or_else(|| format!("无法确定文件所在目录：{}", path.display()))?;
        let mut command = std::process::Command::new("xdg-open");
        command.arg(directory);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法在文件管理器中显示 {}：{error}", path.display()))
}

/// 处理预览编辑器的消息更新
///
/// 这是预览编辑器模块的主入口函数，负责处理所有编辑器相关的消息，
/// 并返回相应的异步任务（如 LSP 请求、UI 更新等）。
///
/// # 参数
///
/// * `app` - 应用状态的可变引用
/// * `message` - 要处理的预览消息
///
/// # 返回值
///
/// 返回 `Task<Message>`，可能包含：
/// - `Task::none()`: 无需异步操作
/// - `Task::done(...)`: 立即完成的同步消息
/// - 其他异步任务（如 LSP 请求、滚动操作等）
///
/// # 支持的消息类型
///
/// ## 编辑器事件 (EditorEvent)
///
/// 处理键盘输入、鼠标操作等，包括：
/// - LSP 代码补全的导航和确认
/// - 鼠标悬停触发类型提示
/// - Ctrl+点击跳转定义
/// - 字符输入触发补全请求
///
/// ## 上下文菜单操作
///
/// - `ContextMenuOpenForActiveEditor`: 打开右键菜单
/// - `ContextMenuCopy/Cut/Paste/Delete`: 编辑操作
/// - `ContextMenuClose`: 关闭菜单
///
/// ## LSP 相关操作（非 WASM 平台）
///
/// - `LspHoverEntered/Exited`: 悬停提示的鼠标进入/离开
/// - `LspCompletionClosed/Selected/Confirm`: 代码补全的关闭、选择、确认
/// - `LspCompletionNavigateUp/Down`: 补全列表导航
///
/// # 平台差异
///
/// LSP 功能仅在非 WASM 目标平台上可用（通过 `#[cfg(not(target_arch = "wasm32"))]` 标记）。
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
    match message {
        PreviewMessage::EditorEvent(evt) => {
            match &evt {
                iced_code_editor::Message::CustomContextMenuAction(id) => {
                    return match id.as_str() {
                        APP_CONTEXT_MENU_INSERT_SELECTION => Task::done(Message::Chat(
                            crate::app::message::ChatMessage::InsertSelected,
                        )),
                        APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS => Task::done(Message::Chat(
                            crate::app::message::ChatMessage::InsertSelectionPositions,
                        )),
                        unknown => {
                            tracing::warn!(
                                action_id = unknown,
                                "忽略未知的编辑器自定义右键菜单动作"
                            );
                            Task::none()
                        }
                    };
                }
                iced_code_editor::Message::RevealInFileManager => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let Some(path) = app.active_preview_path.as_deref() else {
                            return app.show_error_toast("无法在文件管理器中显示：没有活动文件");
                        };
                        if let Err(error) = reveal_in_file_manager(path) {
                            return app.show_error_toast(error);
                        }
                    }
                    return Task::none();
                }
                _ => {}
            }

            // 更新上下文选择目标（用于右键菜单操作选中文本）
            update_context_target_from_editor_event(app, &evt);

            // ===== LSP 代码补全交互处理（非 WASM 平台） =====
            #[cfg(not(target_arch = "wasm32"))]
            {
                if matches!(&evt, iced_code_editor::Message::ContextMenuRequested(_)) {
                    lsp::clear_lsp_hover(app);
                    lsp::clear_lsp_completion(app, true);
                }

                // 关闭搜索框时，如果补全列表可见，则清除补全
                if matches!(&evt, iced_code_editor::Message::CloseSearch)
                    && app.lsp_overlay.completion_visible
                {
                    lsp::clear_lsp_completion(app, false);
                    return Task::none();
                }

                // 当补全列表可见且未被抑制时，处理导航键
                if app.lsp_overlay.completion_visible
                    && !app.lsp_overlay.completion_suppressed
                    && !app.lsp_overlay.completion_items.is_empty()
                {
                    match &evt {
                        iced_code_editor::Message::ArrowKey(direction, false) => {
                            use iced_code_editor::ArrowDirection;
                            match direction {
                                // 上下箭头键导航补全列表
                                ArrowDirection::Up => {
                                    return Task::done(Message::Preview(
                                        PreviewMessage::LspCompletionNavigateUp,
                                    ));
                                }
                                ArrowDirection::Down => {
                                    return Task::done(Message::Preview(
                                        PreviewMessage::LspCompletionNavigateDown,
                                    ));
                                }
                                // 左右箭头键关闭补全列表
                                ArrowDirection::Left | ArrowDirection::Right => {
                                    lsp::clear_lsp_completion(app, false);
                                }
                            }
                        }
                        // 回车键确认选择补全项
                        iced_code_editor::Message::Enter => {
                            return Task::done(Message::Preview(
                                PreviewMessage::LspCompletionConfirm,
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // 将事件传递给编辑器处理
            app.focus_area = FocusArea::Preview;

            if let Some(path) = app.active_preview_path.as_deref()
                && let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path)
            {
                let was_dirty = tab.is_dirty;
                let may_change_content = editor_message_may_change_content(&evt);
                let was_modified = may_change_content && tab.editor.inner.is_modified();
                let task = tab
                    .editor
                    .inner
                    .update(&evt)
                    .map(|e| Message::Preview(PreviewMessage::EditorEvent(e)));
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
                        app.preview_auto_save_mode,
                        PreviewAutoSaveMode::AfterDelay | PreviewAutoSaveMode::OnFocusChange
                    ) {
                    let revision = tab.auto_save_revision;
                    let auto_save_path = path.to_string();
                    Some(crate::app::message::after(
                        std::time::Duration::from_millis(700),
                        Message::Preview(PreviewMessage::AutoSaveDelayElapsed {
                            path: auto_save_path,
                            revision,
                        }),
                    ))
                } else {
                    None
                };

                // ===== LSP 高级功能处理（非 WASM 平台） =====
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // 鼠标悬停：触发 LSP 类型/文档提示
                    if let iced_code_editor::Message::MouseHover(point) = &evt {
                        // 仅在上下文菜单和导航弹窗都未显示时处理
                        if !app.show_preview_context_menu && app.preview_nav_popup.is_none() {
                            tracing::debug!("[LSP HOVER] MouseHover at point={:?}", point);
                            lsp::handle_lsp_hover_from_mouse(app, *point);
                        }
                    }

                    // 鼠标释放：设置悬停提示的自动隐藏倒计时
                    if let iced_code_editor::Message::MouseRelease = &evt
                        && app.lsp_overlay.hover_visible
                    {
                        // 禁用交互模式，400ms 后自动隐藏
                        app.lsp_overlay.hover_interactive = false;
                        app.lsp_hover_hide_deadline =
                            Some(std::time::Instant::now() + std::time::Duration::from_millis(400));
                    }

                    // Ctrl+点击：跳转到定义
                    if let iced_code_editor::Message::JumpClick(point) = &evt {
                        // 如果上下文菜单或导航弹窗已打开，不处理跳转
                        if app.show_preview_context_menu || app.preview_nav_popup.is_some() {
                            return task;
                        }
                        // 请求 LSP 定义跳转
                        if let Some(path) = app.active_preview_path.clone()
                            && let Some(tab) = app
                                .preview_tabs
                                .iter_mut()
                                .find(|t| t.path.as_str() == path.as_str())
                        {
                            tab.editor.lsp_request_definition_at(*point);
                        }
                    }

                    // 字符输入：更新补全过滤或清除补全
                    if let iced_code_editor::Message::CharacterInput(ch) = &evt
                        && !app.lsp_applying_completion
                    {
                        // 非字母数字下划线字符：清除补全列表
                        if !ch.is_alphanumeric() && *ch != '_' {
                            lsp::clear_lsp_completion(app, false);
                        } else {
                            // 字母数字下划线：取消抑制并更新补全过滤
                            app.lsp_overlay.completion_suppressed = false;

                            // 根据当前输入的单词前缀过滤补全列表
                            if !app.lsp_overlay.all_completions.is_empty()
                                && let Some(path) = app.active_preview_path.as_ref()
                                && let Some(tab) = app.preview_tabs.iter().find(|t| &t.path == path)
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
                                    app.lsp_overlay.completion_filter =
                                        line_content[word_start..col].to_string();
                                    lsp::filter_completions(app);
                                }
                            }
                        }
                    }

                    // 字母数字或下划线输入：触发 LSP 补全请求
                    match &evt {
                        iced_code_editor::Message::CharacterInput(ch)
                            if ch.is_alphanumeric() || *ch == '_' =>
                        {
                            if let Some(path) = app.active_preview_path.clone()
                                && let Some(tab) = app
                                    .preview_tabs
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
            if let Some(path) = app.active_preview_path.clone() {
                super::dismiss_preview_popup_menus(app);
                app.git_diff_context_menu = None;

                // 检查是否需要重置选择目标为全选（1,1 到 1,1 表示无选择）
                let should_reset_target = !matches!(
                    app.preview_context_target,
                    Some((ref p, start_line, start_col, end_line, end_col))
                        if *p == path && (start_line, start_col, end_line, end_col) != (1, 1, 1, 1)
                );

                if should_reset_target {
                    app.preview_context_target = Some((path, 1, 1, 1, 1));
                }

                // 清除 LSP 相关浮层（非 WASM 平台）
                #[cfg(not(target_arch = "wasm32"))]
                {
                    lsp::clear_lsp_hover(app);
                    lsp::clear_lsp_completion(app, true);
                }

                // 显示上下文菜单并记录位置
                let (menu_x, menu_y) =
                    if app.cursor_position.x != 0.0 || app.cursor_position.y != 0.0 {
                        (app.cursor_position.x, app.cursor_position.y)
                    } else {
                        (x, y)
                    };

                app.show_preview_context_menu = true;
                app.preview_context_menu_pos = Some((menu_x, menu_y));
            }
            Task::none()
        }

        // 复制操作
        PreviewMessage::ContextMenuCopy => {
            app.show_preview_context_menu = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(app);
                lsp::clear_lsp_completion(app, false);
            }
            Task::done(Message::Editor(crate::app::message::editor::EditorMessage::Copy))
        }

        // 剪切操作
        PreviewMessage::ContextMenuCut => {
            app.show_preview_context_menu = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(app);
                lsp::clear_lsp_completion(app, false);
            }
            Task::done(Message::Editor(crate::app::message::editor::EditorMessage::Cut))
        }

        // 粘贴操作
        PreviewMessage::ContextMenuPaste => {
            app.show_preview_context_menu = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(app);
                lsp::clear_lsp_completion(app, false);
            }
            Task::done(Message::Editor(crate::app::message::editor::EditorMessage::Paste))
        }

        // 删除操作
        PreviewMessage::ContextMenuDelete => {
            app.show_preview_context_menu = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                lsp::clear_lsp_hover(app);
                lsp::clear_lsp_completion(app, false);
            }
            Task::done(Message::Editor(crate::app::message::editor::EditorMessage::Delete))
        }

        // 关闭上下文菜单
        PreviewMessage::ContextMenuClose => {
            app.show_preview_context_menu = false;
            app.preview_context_target = None;
            app.preview_context_menu_pos = None;
            Task::none()
        }

        PreviewMessage::FullscreenOverlayEntered => {
            app.show_preview_fullscreen_overlay = true;
            Task::none()
        }

        PreviewMessage::FullscreenOverlayExited => {
            app.show_preview_fullscreen_overlay = false;
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::EditorMouseEntered => {
            if app.lsp_overlay_path.as_ref() == app.active_preview_path.as_ref() {
                app.lsp_hover_hide_deadline = None;
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::EditorMouseExited => {
            if app.lsp_overlay_path.as_ref() == app.active_preview_path.as_ref()
                && app.lsp_overlay.hover_visible
                && !app.lsp_overlay.hover_interactive
            {
                app.lsp_hover_hide_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(500));
            }
            Task::none()
        }

        // ===== LSP 悬停提示交互（非 WASM 平台） =====
        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspHoverEntered => {
            // 鼠标进入悬停提示区域：保持显示并启用交互
            app.lsp_overlay.hover_interactive = true;
            app.lsp_hover_hide_deadline = None;
            lsp::focus_hover_overlay()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspHoverExited => {
            // 鼠标离开悬停提示区域：设置 300ms 后自动隐藏
            app.lsp_overlay.hover_interactive = false;
            app.lsp_hover_hide_deadline =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(300));
            Task::none()
        }

        // ===== LSP 代码补全交互（非 WASM 平台） =====
        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionClosed => {
            // 补全列表关闭
            lsp::clear_lsp_completion(app, false);
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionSelected(index) => {
            // 鼠标点击选择补全项
            app.lsp_applying_completion = true;
            let completion = app.lsp_overlay.completion_items.get(index).cloned();
            if let Some(item) = completion {
                lsp::apply_completion(app, &item);
            }
            app.lsp_applying_completion = false;
            lsp::clear_lsp_completion(app, true);
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionNavigateUp => {
            // 向上导航补全列表
            if app.lsp_overlay.completion_visible && !app.lsp_overlay.completion_items.is_empty() {
                // 循环导航：到达顶部时跳转到底部
                if app.lsp_overlay.completion_selected > 0 {
                    app.lsp_overlay.completion_selected -= 1;
                } else {
                    app.lsp_overlay.completion_selected =
                        app.lsp_overlay.completion_items.len() - 1;
                }
                // 返回滚动任务以保持选中项可见
                return lsp::completion_scroll_task(app.lsp_overlay.completion_selected);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionNavigateDown => {
            // 向下导航补全列表
            if app.lsp_overlay.completion_visible && !app.lsp_overlay.completion_items.is_empty() {
                // 循环导航：到达底部时跳转到顶部
                app.lsp_overlay.completion_selected = (app.lsp_overlay.completion_selected + 1)
                    % app.lsp_overlay.completion_items.len();
                // 返回滚动任务以保持选中项可见
                return lsp::completion_scroll_task(app.lsp_overlay.completion_selected);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspCompletionConfirm => {
            // 确认选择补全项（回车键）
            if app.lsp_overlay.completion_visible {
                let selected = app.lsp_overlay.completion_selected;
                app.lsp_applying_completion = true;
                let completion = app.lsp_overlay.completion_items.get(selected).cloned();
                if let Some(item) = completion {
                    lsp::apply_completion(app, &item);
                }
                app.lsp_applying_completion = false;
                lsp::clear_lsp_completion(app, true);
            }
            Task::none()
        }

        #[cfg(not(target_arch = "wasm32"))]
        PreviewMessage::LspToggleDisabled(disabled) => {
            if disabled {
                lsp::disable_lsp(app);
            } else {
                lsp::enable_lsp(app);
            }
            crate::app::config::update_system_settings_config_async(move |system| {
                system.lsp_enabled = !disabled;
            })
        }

        // 处理其他未实现的消息变体，或由其他机制处理的消息（如选择变更）
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;
