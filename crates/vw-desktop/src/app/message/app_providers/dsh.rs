pub use desktop_provider_apps::message::dsh::{Field,change,load,save};
use crate::app::{App,Message,state::app_providers::list::Scope};
pub(crate) fn change_membership(app:&mut App,id:String,add:bool)->iced::Task<Message> {
 let current=super::list::current(app,vw_api_app_identity::AppId::Dsh).ok();
 super::provider_update::change_membership(app,current,id,add)
}
pub(crate) fn membership_changed(app:&mut App,scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_terminal::dsh::DshLiveResult,String>)->iced::Task<Message> {
 let current=super::list::current(app,scope.app).ok();
 super::provider_update::membership_changed(app,current,scope,generation,add,result)
}
