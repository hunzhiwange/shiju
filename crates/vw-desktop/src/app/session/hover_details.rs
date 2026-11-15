//! Read server-owned session metadata only when its sidebar details are requested.
use crate::app::{App, Message};
use iced::Task;

pub use desktop_session::session::hover_details::SessionHoverDetails;
impl App {
    pub(crate) fn load_session_hover_details(
        &mut self,
        session_id: String,
        project_id: String,
        directory: String,
    ) -> Task<Message> {
        if !self.session_ui_projection.hover_details_loading.insert(session_id.clone()) {
            return Task::none();
        }
        let client = crate::app::gateway_client().ok();
        Task::perform(
            desktop_session::session::hover_details::load_session_hover_details(client, session_id, project_id, directory),
            |(id, details)| Message::SessionDetailsLoaded(id, details),
        )
    }
}
