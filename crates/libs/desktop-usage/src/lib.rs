//! Usage center state, local views, and authoritative snapshot operations.
pub mod utils;
pub mod models;
pub mod apps_usage_cards;
pub mod apps_usage_details;
pub mod apps_usage_script;
pub mod refresh;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub endpoint: desktop_settings_account::types::Scope,
    pub project: String,
    pub host: String,
}
#[cfg(test)]
mod apps_usage_details_tests;

pub mod center;
pub mod apps_usage_settings;
pub mod components;
pub mod data;

pub mod view_types;
pub mod layout;
pub mod steps;
pub mod session_menu;
pub mod raw_modal;
pub mod styles;
pub mod entry;
#[cfg(test)]
mod view_test_fixture;

pub mod model_info;

pub mod panel;

#[cfg(test)]
mod desktop_split_30_tests;
