//! 根据内容安全性选择聊天文本的可选中展示策略。

use iced::widget::text_editor;

use crate::app::models::ChatRole;

use super::tool_text_support::is_safe_for_text_editor;

const SESSION_CONTROL_OPEN: &str = "<session_control_selection>";
const SESSION_CONTROL_CLOSE: &str = "</session_control_selection>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 聊天文本根据内容安全性与长度采用的展示方式。
pub(crate) enum SelectableTextPolicy {
    /// 使用单个只读编辑器支持选择。
    SelectableEditor,
    /// 将长文本分段为多个只读编辑器，降低布局成本。
    SegmentedSelectableEditor,
    /// 内容不适合文本编辑器时使用预览并提供复制兜底。
    PreviewWithCopyFallback,
}

/// 在避免 Iced 文本编辑器危险输入的前提下选择展示策略。
pub(crate) fn selectable_text_policy(content: &str, should_segment: bool) -> SelectableTextPolicy {
    if !is_safe_for_text_editor(content) {
        return SelectableTextPolicy::PreviewWithCopyFallback;
    }

    if should_segment {
        SelectableTextPolicy::SegmentedSelectableEditor
    } else {
        SelectableTextPolicy::SelectableEditor
    }
}

/// 为安全且非空的文本创建只读编辑器内容，否则返回 `None`。
pub(crate) fn read_only_editor_content(text: &str) -> Option<text_editor::Content> {
    if text.trim().is_empty()
        || selectable_text_policy(text, false) == SelectableTextPolicy::PreviewWithCopyFallback
    {
        return None;
    }

    Some(text_editor::Content::with_text(text))
}

fn strip_session_control_selection(raw: &str) -> String {
    let Some(open_start) = raw.find(SESSION_CONTROL_OPEN) else {
        return raw.to_string();
    };
    let content_start = open_start + SESSION_CONTROL_OPEN.len();
    let Some(close_offset) = raw[content_start..].find(SESSION_CONTROL_CLOSE) else {
        return raw.to_string();
    };
    let close_start = content_start + close_offset;
    let close_end = close_start + SESSION_CONTROL_CLOSE.len();

    let mut cleaned = String::with_capacity(raw.len().saturating_sub(close_end - open_start));
    cleaned.push_str(&raw[..open_start]);
    cleaned.push_str(&raw[close_end..]);
    cleaned.trim().to_string()
}

/// 清理会话控制标记，并按角色返回实际可见的编辑器文本。
pub(crate) fn visible_editor_text_for_message(
    role: ChatRole,
    content: &str,
    assistant_visible: &str,
    tool_display_text: &str,
) -> String {
    let raw = match role {
        ChatRole::Assistant => assistant_visible,
        ChatRole::Tool => tool_display_text,
        ChatRole::User | ChatRole::System | ChatRole::Developer => content,
    };
    let (without_attachments, _) =
        crate::app::components::input_panel::attachment::parse_attachment_markers(raw);
    strip_session_control_selection(&without_attachments)
}
