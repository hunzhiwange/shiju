use crate::app::Message;
use crate::app::message::app_providers::list::{self, ListMessage};
pub use desktop_settings_apps::aggregation::clients::*;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Aggregation(super::Event::Clients(event))) }
