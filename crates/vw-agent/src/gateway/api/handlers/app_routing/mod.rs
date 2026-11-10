mod config_api;
mod lifecycle_api;

pub(crate) use config_api::{get_app_config, get_config, put_app_config, put_config};
pub(crate) use lifecycle_api::{configure, health, start, status, stop};

#[cfg(test)]
mod config_api_tests;
#[cfg(test)]
mod lifecycle_api_tests;
