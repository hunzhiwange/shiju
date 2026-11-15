pub use desktop_provider_apps::message::claude::{Field,change,load,save};
pub(crate) fn finish(
    state: &mut crate::app::state::app_providers::list::ListState,
    scope: &desktop_provider_apps::Scope,
    generation: u64,
    result: &Result<ClaudeLiveResult, String>,
) -> bool {
 desktop_provider_apps::message::claude::finish(&mut desktop_provider_apps::state::edit::ProviderEditState { scope: state.scope.as_ref(), generation: state.generation, form: &mut state.form }, scope, generation, result)
}
use vw_api_app_provider_base::claude::ClaudeLiveResult;
