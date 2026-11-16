use crate::app::Message;
pub(super) use desktop_chat_tools::tools::file_link::file_name_only;
pub(super) fn file_name_link<'a>(display: &str, path: &str) -> iced::Element<'a, Message> {
 desktop_chat_tools::tools::file_link::file_name_link::<Message>(display,path).map(super::card_adapter::map)
}
pub(super) fn file_path_tooltip<'a>(path: String) -> iced::Element<'a, Message> {
 desktop_chat_tools::tools::file_link::file_path_tooltip::<Message>(path).map(super::card_adapter::map)
}
