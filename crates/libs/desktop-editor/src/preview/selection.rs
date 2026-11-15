//! 编辑事件分类、光标定位与选择范围。
use unicode_width::UnicodeWidthChar;

pub fn editor_message_may_change_content(message: &iced_code_editor::Message) -> bool {
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
            | iced_code_editor::Message::MoveLineUp
            | iced_code_editor::Message::MoveLineDown
            | iced_code_editor::Message::DuplicateLineUp
            | iced_code_editor::Message::DuplicateLineDown
            | iced_code_editor::Message::ToggleComment
            // Vim keys depend on parser state and can mutate the document.
            | iced_code_editor::Message::VimKey(_)
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
pub fn cursor_from_point(
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
pub fn normalize_pos(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) { (a, b) } else { (b, a) }
}

/// 从编辑器事件中更新上下文选择目标
///
/// 根据鼠标点击和拖拽事件，更新应用状态中的 `preview_context_target`，
/// 用于跟踪用户选择的文本范围（支持右键菜单操作选中文本）。
///
/// # 参数
///
/// * 状态由调用方显式借用，范围仅限活动预览、编辑器与选择目标。
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
pub fn update_context_target_from_editor_event(
    active_preview_path: &Option<String>,
    preview_tabs: &[super::PreviewTab],
    preview_context_target: &mut Option<(String, usize, usize, usize, usize)>,
    evt: &iced_code_editor::Message,
) {
    // 获取当前活动的预览文件路径
    let Some(path) = active_preview_path.clone() else {
        return;
    };

    // 查找对应的预览标签页
    let Some(tab) = preview_tabs.iter().find(|t| t.path == path) else {
        return;
    };

    match evt {
        iced_code_editor::Message::MouseClick(point) => {
            // 鼠标单击：设置光标位置为单点选择（起点=终点）
            if let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) {
                (*preview_context_target) = Some((path, line, col, line, col));
            }
        }
        iced_code_editor::Message::ContextMenuRequested(point) => {
            let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) else {
                return;
            };
            let clicked_inside_active_selection = matches!(
                (*preview_context_target),
                Some((ref target_path, start_line, start_col, end_line, end_col))
                    if *target_path == path
                        && (start_line, start_col) != (end_line, end_col)
                        && (start_line, start_col) <= (line, col)
                        && (line, col) <= (end_line, end_col)
            );
            if !clicked_inside_active_selection {
                (*preview_context_target) = Some((path, line, col, line, col));
            }
        }
        iced_code_editor::Message::MouseDrag(point) => {
            // 鼠标拖拽：更新选择范围
            let Some((line, col)) = cursor_from_point(&tab.editor.inner, *point) else {
                return;
            };

            // 检查是否存在有效的选择起点，且路径匹配
            if let Some((target_path, start_line, start_col, _, _)) =
                preview_context_target.clone()
                && target_path == path
            {
                // 规范化选择范围（确保 start <= end）
                let ((sl, sc), (el, ec)) = normalize_pos((start_line, start_col), (line, col));
                (*preview_context_target) = Some((path, sl, sc, el, ec));
            } else {
                // 无有效起点或路径不匹配，从当前位置开始新选择
                (*preview_context_target) = Some((path, line, col, line, col));
            }
        }
        _ => {}
    }
}

#[cfg(test)] #[path = "editor_tests.rs"] mod editor_tests;
