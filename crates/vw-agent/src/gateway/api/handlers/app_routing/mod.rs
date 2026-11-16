mod lifecycle_api;

#[path = "../agent_apps_high_availability.rs"]
pub(crate) mod agent_apps_high_availability;

#[cfg(test)]
pub(crate) use lifecycle_api::configure_with_policy;
pub(crate) use lifecycle_api::{RoutingControl, configure};

#[cfg(test)]
pub(super) mod lifecycle_api_tests;
