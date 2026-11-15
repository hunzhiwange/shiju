use super::*;

pub fn build_embedded_text_editor<'a, F>(
    content: &'a text_editor::Content,
    placeholder: &'static str,
    map_message: F,
    height: f32,
) -> Element<'a, Message>
where
    F: Fn(text_editor::Action) -> WorkflowMessage + 'static + Copy,
{
    text_editor(content)
        .placeholder(placeholder)
        .on_action(move |action| map_message(action))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .height(Length::Fixed(height))
        .style(editor_style)
        .into()
}
