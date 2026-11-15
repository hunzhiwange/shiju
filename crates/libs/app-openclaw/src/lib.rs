//! OpenClaw configuration tools and guarded publication.
#![cfg(target_os = "macos")]

pub mod openclaw_defaults;
pub mod openclaw_env;
pub mod openclaw_health;
pub mod openclaw_model;
pub mod openclaw_tools;

pub use app_openclaw_config_rules::catalog as openclaw_catalog;

#[cfg(test)]
mod openclaw_env_tests;
#[cfg(test)]
mod openclaw_health_tests;
#[cfg(test)]
mod openclaw_model_tests;
#[cfg(test)]
mod openclaw_tools_tests;
