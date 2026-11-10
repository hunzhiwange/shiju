mod config_api;
mod lifecycle_api;

#[path = "../agent_apps_high_availability.rs"]
pub(crate) mod agent_apps_high_availability;

pub(crate) use config_api::{get_app_config, get_config, put_app_config, put_config};
pub(crate) use lifecycle_api::{RoutingControl, configure, health, start, status, stop};

#[cfg(test)]
mod config_api_tests;
#[cfg(test)]
pub(super) mod lifecycle_api_tests;
