pub use desktop_provider_apps::message::preview::{Event};
use crate::app::{App,Message};
use iced::Task;
pub fn update(app:&mut App,event:Event)->Task<Message>{
 let current=app.app_providers.scope.as_ref().ok_or_else(||"无法连接网关".to_string()).and_then(|s|super::list::current(app,s.app));
 super::provider_update::run(app,|state,token,snapshot|desktop_provider_apps::message::preview::update(state,current,token,snapshot,event))
}

pub fn message(event:Event)->Message {super::list::message(super::list::ListMessage::Preview(event))}
