use super::selectable_text::*;
use session_ui_types::ChatRole;
#[test]
fn selectable_text_policy_prefers_editor_for_safe_text() {
    assert_eq!(
        selectable_text_policy("safe text\nsecond line", false),
        SelectableTextPolicy::SelectableEditor
    );
    assert_eq!(
        selectable_text_policy(&"line\n".repeat(120), true),
        SelectableTextPolicy::SegmentedSelectableEditor
    );
    assert!(read_only_editor_content("safe text").is_some());
}

#[test]
fn visible_editor_text_uses_role_specific_source_and_strips_session_control() {
    assert_eq!(
        visible_editor_text_for_message(
            ChatRole::Assistant,
            "stored assistant text",
            "visible assistant text",
            "tool text",
        ),
        "visible assistant text"
    );
    assert_eq!(
        visible_editor_text_for_message(ChatRole::Tool, "stored tool text", "assistant", "shown"),
        "shown"
    );
    assert_eq!(
        visible_editor_text_for_message(
            ChatRole::User,
            "keep<session_control_selection>hidden</session_control_selection>tail",
            "assistant",
            "tool",
        ),
        "keeptail"
    );
    assert_eq!(
        visible_editor_text_for_message(
            ChatRole::System,
            "unclosed <session_control_selection>context",
            "assistant",
            "tool",
        ),
        "unclosed <session_control_selection>context"
    );
}
