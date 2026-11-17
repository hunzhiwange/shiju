#[cfg(test)]
pub(crate) fn finish(
    state: &mut crate::app::state::app_providers::list::ListState,
    scope: &desktop_provider_apps::Scope,
    generation: u64,
    result: &Result<CodexLiveResult, String>,
) -> bool {
 desktop_provider_apps::message::codex::finish(&mut desktop_provider_apps::state::edit::ProviderEditState { scope: state.scope.as_ref(), generation: state.generation, form: &mut state.form }, scope, generation, result)
}

#[cfg(test)]
use vw_api_app_provider_profiles::codex_catalog::CodexLiveResult;
#[cfg(test)]
pub use desktop_provider_apps::message::codex::Field;
#[cfg(test)]
pub use desktop_provider_apps::message::codex::change;
#[cfg(test)]
pub use desktop_provider_apps::message::codex::load;
#[cfg(test)]
pub use desktop_provider_apps::message::codex::save;
