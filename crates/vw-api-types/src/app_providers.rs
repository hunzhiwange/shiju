//! Native application provider compatibility exports.
pub use vw_api_app_provider_base::{ProviderProtocol, AppProviderWrite, AppProviderSummary, ProviderLaunchRequest};

pub mod catalog;
#[cfg(test)]
mod catalog_tests;



pub mod delete;
#[cfg(test)]
mod delete_tests;

pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;

pub mod presentation;
#[cfg(test)]
mod presentation_tests;

pub mod endpoints;
#[cfg(test)]
mod endpoints_tests;

pub mod icons;
#[cfg(test)]
mod icons_tests;

pub mod universal;
#[cfg(test)]
mod universal_tests;

pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;

pub mod paths;
#[cfg(test)]
mod paths_tests;

pub mod preview;
#[cfg(test)]
mod preview_tests;


pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;

pub mod snippets;
#[cfg(test)]
mod snippets_tests;


#[cfg(test)]
mod codex_tests;

pub mod codex_catalog;

pub mod gemini;

pub mod claude_desktop;
pub mod grok;
#[cfg(test)]
mod grok_tests;


pub mod desktop_mapping;

pub mod hermes;
#[cfg(test)]
mod hermes_tests;
pub mod openclaw;
#[cfg(test)]
mod openclaw_tests;
pub mod opencode;
#[cfg(test)]
mod opencode_tests;

pub mod pi;

pub mod omp;

pub mod mcode;

pub mod kimi;

pub mod dsh;

pub mod agy;

pub mod import;
#[cfg(test)]
mod import_tests;

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
