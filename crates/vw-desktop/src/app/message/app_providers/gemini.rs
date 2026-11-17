#[cfg(test)]
pub(crate) fn finish(state: &mut crate::app::state::app_providers::list::ListState, scope: &desktop_provider_apps::Scope, generation: u64, result: &Result<vw_api_app_provider_profiles::gemini::GeminiLiveResult,String>) -> bool {
 desktop_provider_apps::message::gemini::finish(&mut desktop_provider_apps::state::edit::ProviderEditState {scope:state.scope.as_ref(),generation:state.generation,form:&mut state.form},scope,generation,result)
}

#[cfg(test)]
pub use desktop_provider_apps::message::gemini::Field;
#[cfg(test)]
pub use desktop_provider_apps::message::gemini::change;
#[cfg(test)]
pub use desktop_provider_apps::message::gemini::load;
#[cfg(test)]
pub use desktop_provider_apps::message::gemini::save;
