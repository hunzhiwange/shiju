pub use desktop_provider_apps::message::icons::Event;
use crate::app::{App,Message};
pub(crate) fn message(event:Event)->Message { super::list::message(super::list::ListMessage::Icons(event)) }
pub(crate) fn update(app:&mut App,event:Event)->iced::Task<Message>{
 let current=app.app_providers.scope.as_ref().ok_or_else(||"无法连接网关".to_string()).and_then(|s|super::list::current(app,s.app));
 super::provider_update::icons(app,current,event)
}
