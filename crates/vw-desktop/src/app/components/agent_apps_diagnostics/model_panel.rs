use crate::app::Message;
use crate::app::message::agent_apps_diagnostics::{model, model_state::ModelState};
use iced::Element;
pub(crate) fn overlay<'a>(state: &'a ModelState, base: Element<'a, Message>) -> Element<'a, Message> {
    if state.open.is_none() { return base; }
    iced::widget::stack![base, desktop_apps_ui::diagnostics::model_panel::overlay(state, iced::widget::Space::new().into()).map(model::message)].into()
}
