//! Native application provider compatibility exports.
pub use vw_api_app_provider_base::{ProviderProtocol, AppProviderWrite, AppProviderSummary, ProviderLaunchRequest};

pub mod catalog;



pub mod delete;

pub mod duplicate;

pub mod presentation;

pub mod endpoints;

pub mod icons;

pub mod universal;

pub mod universal_sync;

pub mod paths;
#[cfg(test)]
mod paths_tests;

pub mod preview;


pub mod editor_merge;

pub mod snippets;


#[cfg(test)]
mod codex_tests;

pub mod codex_catalog;

pub mod gemini;

pub mod claude_desktop;
pub mod grok;


pub mod desktop_mapping;

pub mod hermes;
pub mod openclaw;
pub mod opencode;

pub mod pi;

pub mod omp;

pub mod mcode;

pub mod kimi;

pub mod dsh;

pub mod agy;

pub mod import;

pub mod mode_state;
#[cfg(test)]
mod mode_state_tests;

pub mod mode_apply;
#[cfg(test)]
mod mode_apply_tests;

pub mod env_diagnose;
#[cfg(test)]
mod env_diagnose_tests;

pub mod env_clean;
#[cfg(test)]
mod env_clean_tests;

pub mod env_restore;
#[cfg(test)]
mod env_restore_tests;

pub mod operations;
#[cfg(test)]
mod operations_tests;


pub mod request_options;
