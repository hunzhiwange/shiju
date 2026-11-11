mod config_api;
mod lifecycle_api;
mod outbound_api;
mod outbound_probe;

#[path = "../agent_apps_high_availability.rs"]
pub(crate) mod agent_apps_high_availability;

pub(crate) use config_api::{get_app_config, get_config, put_app_config, put_config};
pub(crate) use lifecycle_api::{RoutingControl, configure, health, start, status, stop};
pub(crate) use outbound_api::{get_egress, put_egress};
pub(crate) use outbound_probe::{
    cancel_outbound_probe, outbound_probe_status, start_outbound_probe,
};

#[cfg(test)]
mod config_api_tests;
#[cfg(test)]
pub(super) mod lifecycle_api_tests;

#[cfg(test)]
mod outbound_api_tests;

#[cfg(test)]
mod outbound_probe_tests;
