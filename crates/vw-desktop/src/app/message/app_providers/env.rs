pub use desktop_provider_apps::message::env::Event;
use crate::app::{App,Message};
pub(crate) fn message(event:Event)->Message { super::list::message(super::list::ListMessage::Env(event)) }
pub(crate) fn update(app:&mut App,event:Event)->iced::Task<Message>{
 let current=app.app_providers.scope.as_ref().and_then(|s|super::list::current(app,s.app).ok());
 super::provider_update::env(app,current,event)
}
