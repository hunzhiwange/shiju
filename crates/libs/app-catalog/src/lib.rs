//! Application catalog, ACP projections and pure execution-host preference rules.
pub mod acp_projection;
#[cfg(test)]
mod acp_projection_tests;
pub mod catalog;
#[cfg(test)]
mod catalog_tests;
pub mod preferences;
#[cfg(test)]
mod preferences_tests;
pub mod wsl;
#[cfg(test)]
mod wsl_tests;
