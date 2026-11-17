#[cfg(test)]
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Aggregation(super::Event::Publish(event))) }

#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use crate::app::message::app_providers::list;
#[cfg(test)]
use crate::app::message::app_providers::list::ListMessage;
#[cfg(test)]
pub use desktop_settings_apps::aggregation::publish::*;
