pub use desktop_provider_apps::{Scope, state::list::{Snapshot, Editor}};
#[cfg(not(target_arch = "wasm32"))]
type Usage = crate::app::components::apps_usage_cards::Cards;
#[cfg(target_arch = "wasm32")]
type Usage = ();
/// Desktop fixes the cross-domain slots; lifecycle logic belongs to provider apps.
pub(crate) type ListState = desktop_provider_apps::state::list::ListState<
    super::leave::LeaveState,
    crate::app::message::agent_apps_high_availability::State,
    crate::app::message::settings::aggregation_list::Form,
    crate::app::message::settings::aggregation_subagent::Form,
    crate::app::message::settings::aggregation_pi::Form,
    crate::app::message::agent_apps_diagnostics::endpoint_state::State,
    crate::app::message::settings::aggregation_omp::Form,
    crate::app::state::aggregation_omo::Form,
    Usage,
>;
