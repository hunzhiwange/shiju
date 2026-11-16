mod tool_renderer;
mod tool_text_support;

use iced::{Theme, widget::text_editor};
use std::collections::HashMap;

#[derive(Default)]
struct TextEditors {
    chat_message_editors: Vec<text_editor::Content>,
    chat_special_text_editors: HashMap<u64, text_editor::Content>,
    chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    chat_think_editors: HashMap<u64, text_editor::Content>,
    chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
}
impl TextEditors {
    fn state(&self) -> crate::tool_text_support::ToolTextState<'_> {
        crate::tool_text_support::ToolTextState {
            chat_message_editors: &self.chat_message_editors,
            chat_special_text_editors: &self.chat_special_text_editors,
            chat_tool_text_editors: &self.chat_tool_text_editors,
            chat_think_editors: &self.chat_think_editors,
            chat_markdown_block_editors: &self.chat_markdown_block_editors,
            markdown_selection: None,
            theme: Theme::Light,
        }
    }
}
