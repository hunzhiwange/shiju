use crate::app::models::ChatRole;
pub use desktop_chat_view::selectable_text::{
    SelectableTextPolicy, read_only_editor_content, selectable_text_policy,
};
pub(crate) fn visible_editor_text_for_message(
    role: ChatRole,
    content: &str,
    assistant_visible: &str,
    tool_display_text: &str,
) -> String {
    let raw = desktop_chat_view::selectable_text::visible_editor_source(
        role,
        content,
        assistant_visible,
        tool_display_text,
    );
    let (cleaned, _) =
        crate::app::components::input_panel::attachment::parse_attachment_markers(raw);
    desktop_chat_view::selectable_text::visible_editor_text_for_message(
        role, &cleaned, &cleaned, &cleaned,
    )
}
