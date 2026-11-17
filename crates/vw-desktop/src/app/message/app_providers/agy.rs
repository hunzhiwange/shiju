#[cfg(test)]
pub(crate) fn finish(
    state: &mut crate::app::state::app_providers::list::ListState,
    scope: &desktop_provider_apps::Scope,
    generation: u64,
    result: &Result<(), String>,
) -> bool {
 desktop_provider_apps::message::agy::finish(&mut desktop_provider_apps::state::edit::ProviderEditState { scope: state.scope.as_ref(), generation: state.generation, form: &mut state.form }, scope, generation, result)
}

#[cfg(test)]
pub use desktop_provider_apps::message::agy::Field;
#[cfg(test)]
pub use desktop_provider_apps::message::agy::change;
#[cfg(test)]
pub use desktop_provider_apps::message::agy::load;
#[cfg(test)]
pub use desktop_provider_apps::message::agy::save;
