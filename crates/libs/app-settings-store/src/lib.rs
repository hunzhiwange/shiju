//! Scoped application configuration persistence and shared post-commit hints.
pub mod settings_store;
pub mod agent_apps_high_availability_store;
pub mod agent_apps_skills_store;
pub mod change_events;

#[cfg(test)]
mod settings_store_tests;
#[cfg(test)]
mod agent_apps_high_availability_store_tests;
#[cfg(test)]
mod agent_apps_skills_store_tests;
