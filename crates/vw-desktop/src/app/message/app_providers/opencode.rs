pub use desktop_provider_apps::message::opencode::{Field,change,load,save};
use crate::app::{App,Message};
use iced::Task;
use desktop_provider_apps::Scope;
pub fn finish(state:&mut crate::app::state::app_providers::list::ListState,scope:&Scope,generation:u64,result:&Result<vw_api_app_provider_extended::opencode::OpenCodeLiveResult,String>)->bool {
 desktop_provider_apps::message::opencode::finish(&mut desktop_provider_apps::state::edit::ProviderEditState {scope:state.scope.as_ref(),generation:state.generation,form:&mut state.form},scope,generation,result)
}
pub fn change_membership(app:&mut App,id:String,add:bool)->Task<Message>{
 let current=super::list::current(app,vw_api_types::agent_apps::AppId::OpenCode).ok();
 super::provider_update::run(app,|state,token,snapshot|desktop_provider_apps::message::opencode::change_membership(state,current,token,snapshot,id,add))
}
pub fn membership_changed(app:&mut App,scope:Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_extended::opencode::OpenCodeLiveResult,String>)->Task<Message>{
 let current=super::list::current(app,scope.app).ok();
 super::provider_update::run(app,|state,_,_|desktop_provider_apps::message::opencode::membership_changed(state,current,scope,generation,add,result))
}
