//! Application-provider services and validation exports.
pub use app_provider_validation::{validate_identity, validate_profile};

pub mod catalog;
#[cfg(all(test, target_os = "macos"))]
mod catalog_tests;

pub mod failover;

pub mod create;
#[cfg(test)]
mod create_tests;

pub mod update;
#[cfg(test)]
mod update_tests;

pub mod delete;
#[cfg(all(test, target_os = "macos"))]
mod delete_tests;

pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;

pub mod presentation;
#[cfg(test)]
mod presentation_tests;

pub use app_provider_validation::endpoints;

pub mod icons;
#[cfg(test)]
mod icons_tests;

pub mod universal;
#[cfg(test)]
mod universal_tests;

pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;

pub use app_config_paths as paths;

pub mod live;

pub mod preview;
#[cfg(all(test, target_os = "macos"))]
mod preview_tests;

mod projection;
pub(crate) use projection::fields;

pub use app_provider_validation::validation;
#[cfg(test)]
mod validation_tests;

pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;

pub mod snippets;
#[cfg(test)]
mod snippets_tests;

pub mod claude;
#[cfg(test)]
mod claude_tests;

#[cfg(target_os = "macos")]
pub mod claude_login;
#[cfg(all(test, target_os = "macos"))]
mod claude_login_tests;

pub mod codex;
#[cfg(test)]
mod codex_tests;

#[cfg(target_os = "macos")]
pub mod codex_login;
#[cfg(target_os = "macos")]
pub(crate) mod codex_login_material;
#[cfg(target_os = "macos")]
mod codex_login_policy;
#[cfg(all(test, target_os = "macos"))]
mod codex_login_tests;

#[cfg(target_os = "macos")]
pub mod codex_catalog;
#[cfg(all(test, target_os = "macos"))]
mod codex_catalog_tests;

pub mod gemini;
#[cfg(all(test, target_os = "macos"))]
mod gemini_tests;

pub mod claude_desktop;
pub mod grok;
#[cfg(all(test, target_os = "macos"))]
mod grok_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_desktop_tests;

pub mod desktop_mapping;
#[cfg(test)]
mod desktop_mapping_tests;

#[cfg(target_os = "macos")]
pub mod hermes;
#[cfg(all(test, target_os = "macos"))]
mod hermes_tests;
#[cfg(target_os = "macos")]
pub mod openclaw;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_tests;
#[cfg(target_os = "macos")]
pub mod opencode;
#[cfg(all(test, target_os = "macos"))]
mod opencode_tests;

#[cfg(target_os = "macos")]
mod opencode_schema;

#[cfg(target_os = "macos")]
pub mod pi;
#[cfg(all(test, target_os = "macos"))]
mod pi_tests;

#[cfg(target_os = "macos")]
pub mod omp;
#[cfg(all(test, target_os = "macos"))]
mod omp_tests;

#[cfg(target_os = "macos")]
pub mod mcode;
#[cfg(all(test, target_os = "macos"))]
mod mcode_tests;

#[cfg(target_os = "macos")]
pub mod kimi;
#[cfg(all(test, target_os = "macos"))]
mod kimi_tests;

#[cfg(target_os = "macos")]
pub mod dsh;
#[cfg(all(test, target_os = "macos"))]
mod dsh_tests;

#[cfg(target_os = "macos")]
pub mod agy;
#[cfg(all(test, target_os = "macos"))]
mod agy_tests;

pub mod import;
#[cfg(test)]
mod import_tests;

#[cfg(target_os = "macos")]
pub mod mode_state;
#[cfg(all(test, target_os = "macos"))]
mod mode_state_tests;

#[cfg(target_os = "macos")]
pub mod mode_apply;
#[cfg(all(test, target_os = "macos"))]
mod mode_apply_tests;

pub mod env_diagnose;
#[cfg(all(test, target_os = "macos"))]
mod env_diagnose_tests;

pub mod env_clean;
#[cfg(all(test, target_os = "macos"))]
mod env_clean_tests;

pub mod env_restore;
#[cfg(all(test, target_os = "macos"))]
mod env_restore_tests;

#[cfg(target_os = "macos")]
mod env_backup;

pub mod operations;
#[cfg(test)]
mod operations_tests;

pub mod basic;

pub use app_provider_validation::request_options;

#[cfg(target_os = "macos")]
mod mode_apply_publication;

#[cfg(all(test, target_os = "macos"))]
mod mode_apply_publication_tests;
