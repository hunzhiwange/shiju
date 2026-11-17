#[cfg(test)]
pub(crate) fn finish(state: &mut crate::app::state::app_providers::list::ListState, scope: &desktop_provider_apps::Scope, generation: u64, result: &Result<vw_api_app_provider_extended::hermes::HermesLiveResult,String>) -> bool {
 desktop_provider_apps::message::hermes::finish(&mut desktop_provider_apps::state::edit::ProviderEditState {scope:state.scope.as_ref(),generation:state.generation,form:&mut state.form},scope,generation,result)
}
pub(crate) fn change_membership(app:&mut crate::app::App,id:String,add:bool)->iced::Task<crate::app::Message>{
 let current=super::list::current(app,vw_api_app_identity::AppId::Hermes).ok();
 super::provider_update::hermes_change(app,current,id,add)
}
pub(crate) fn membership_changed(app:&mut crate::app::App,scope:desktop_provider_apps::Scope,generation:u64,add:bool,result:Result<vw_api_app_provider_extended::hermes::HermesLiveResult,String>)->iced::Task<crate::app::Message>{
 let current=super::list::current(app,scope.app).ok(); super::provider_update::hermes_changed(app,current,scope,generation,add,result)
}

#[cfg(test)]
pub use desktop_provider_apps::message::hermes::Field;
#[cfg(test)]
pub use desktop_provider_apps::message::hermes::change;
#[cfg(test)]
pub use desktop_provider_apps::message::hermes::load;
#[cfg(test)]
pub use desktop_provider_apps::message::hermes::save;
