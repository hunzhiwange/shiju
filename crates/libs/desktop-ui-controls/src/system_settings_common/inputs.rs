use super::styles::{settings_text_editor_style, settings_text_input_style};
use super::utils::{SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE};
use iced::advanced::text::highlighter::PlainText;
use iced::widget::{TextEditor, TextInput, text_editor, text_input};

/// 构建使用设置样式的单行输入框，由调用方绑定消息和遮罩。
pub fn settings_text_input<'a, Message: Clone>(
    placeholder: &str,
    value: &str,
) -> TextInput<'a, Message> {
    text_input(placeholder, value)
        .font(iced::Font::DEFAULT)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
}

/// 构建使用设置样式的多行输入框，由调用方绑定编辑消息和尺寸。
pub fn settings_text_editor<Message: Clone>(
    content: &text_editor::Content,
) -> TextEditor<'_, PlainText, Message> {
    // 设置内容可在编辑中变为中文，不随内容切换为等宽字体。
    text_editor(content)
        .font(iced::Font::DEFAULT)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style)
}
