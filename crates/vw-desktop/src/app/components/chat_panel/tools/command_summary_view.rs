use crate::app::{App, Message};
pub(crate) fn tool_command_summary_view<'a>(app: &'a App, m: usize, items: &[super::CommandItem]) -> Option<iced::Element<'a, Message>> {
 desktop_chat_tools::tools::command_summary_view::tool_command_summary_view(&super::card_adapter::input(app),m,items).map(|e| e.map(super::card_adapter::map_card))
}

#[cfg(test)]
pub(crate) use desktop_chat_tools::tools::command_summary_view::COMMAND_LIST_TOP_SPACING;
#[cfg(test)]
pub(crate) use desktop_chat_tools::tools::command_summary_view::is_command_tool;
