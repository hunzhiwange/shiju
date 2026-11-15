//! Domain projections for explicitly selected migration data.
use app_data_format::{portable_policy, preview, source};

pub mod mcp;
#[cfg(test)]
mod mcp_tests;
pub mod preferences;
pub mod profiles;
#[cfg(test)]
mod profiles_tests;
pub mod prompts;
#[cfg(test)]
mod prompts_tests;
pub mod providers;
#[cfg(test)]
mod providers_tests;
#[cfg(target_os = "macos")]
pub mod skills;
#[cfg(all(test, target_os = "macos"))]
mod skills_tests;
pub mod usage;
#[cfg(test)]
mod usage_tests;

#[cfg(test)]
mod source_test_support;
