pub use desktop_chat_view::selectable_text::{SelectableTextPolicy, selectable_text_policy, read_only_editor_content};
use crate::app::models::ChatRole;
pub(crate) fn visible_editor_text_for_message(role: ChatRole, content: &str, assistant_visible: &str, tool_display_text: &str) -> String {
    let raw = match role {
        ChatRole::Assistant => assistant_visible,
        ChatRole::Tool => tool_display_text,
        ChatRole::User | ChatRole::System | ChatRole::Developer => content,
    };
    let (cleaned, _) = crate::app::components::input_panel::attachment::parse_attachment_markers(raw);
    desktop_chat_view::selectable_text::visible_editor_text_for_message(role, &cleaned, &cleaned, &cleaned)
}
