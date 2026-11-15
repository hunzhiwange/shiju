pub use desktop_provider_apps::message::desktop::Event;
use crate::app::{App,Message};
pub(crate) fn message(value:Event)->Message { super::list::message(super::list::ListMessage::Desktop(value)) }
pub(crate) fn update(app:&mut App,event:Event)->iced::Task<Message> {
 let current=super::list::current(app,vw_api_app_identity::AppId::ClaudeDesktop).ok();
 let binding=crate::app::message::settings::aggregation_desktop::binding_is_current(app);
 super::provider_update::desktop(app,current,binding,event)
}
