//! Map local workflow page events to desktop effects.
use crate::app::Message;
pub fn view(
    i18n: crate::app::i18n::I18n,
    state: &desktop_workflow_state::state::WorkflowState,
) -> iced::Element<'_, Message> {
    desktop_workflow_ui::page::view(i18n, state).map(|message| {
        use desktop_workflow_ui::page::{PageMessage, ViewMessage};
        match message {
            PageMessage::WorkflowTool(message) => Message::WorkflowTool(message),
            PageMessage::None => Message::None,
            PageMessage::CopyCode(text) => Message::CopyCode(text),
            Pagecrate::app::components::text_editor_context_menu::text_context_message(effect) => {
                crate::app::components::text_editor_context_menu::text_context_message(effect)
            }
            PageMessage::View(ViewMessage::OpenUrlExternal(url)) => {
                Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))
            }
        }
    })
}
