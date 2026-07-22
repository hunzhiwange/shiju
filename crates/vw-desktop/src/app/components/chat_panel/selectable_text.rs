use iced::widget::text_editor;

use crate::app::models::ChatRole;

use super::tool_text_support::is_safe_for_text_editor;

const SESSION_CONTROL_OPEN: &str = "<session_control_selection>";
const SESSION_CONTROL_CLOSE: &str = "</session_control_selection>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectableTextPolicy {
    SelectableEditor,
    SegmentedSelectableEditor,
    PreviewWithCopyFallback,
}

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

pub(crate) fn visible_editor_text_for_message(
    role: ChatRole,
    content: &str,
    assistant_visible: &str,
    tool_display_text: &str,
) -> String {
    let raw = match role {
        ChatRole::Assistant => assistant_visible,
        ChatRole::Tool => tool_display_text,
        ChatRole::User | ChatRole::System => content,
    };
    let (without_attachments, _) =
        crate::app::components::input_panel::attachment::parse_attachment_markers(raw);
    strip_session_control_selection(&without_attachments)
}
