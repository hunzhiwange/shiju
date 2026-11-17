//! Application-provider services and validation exports.
pub use app_provider_validation::{validate_identity, validate_profile};

pub mod catalog;
#[cfg(all(test, target_os = "macos"))]
mod catalog_tests;

pub mod failover;

pub use app_provider_core::create;

pub use app_provider_core::update;

pub mod delete;
#[cfg(all(test, target_os = "macos"))]
mod delete_tests;

pub use app_provider_core::duplicate;

pub use app_provider_core::presentation;

pub use app_provider_validation::endpoints;

pub use app_provider_core::icons;

pub use app_provider_core::universal;

pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;

pub use app_config_paths as paths;

pub mod live;

pub mod preview;
#[cfg(all(test, target_os = "macos"))]
mod preview_tests;

pub(crate) use app_provider_projection::projection;
pub(crate) use projection::fields;

pub use app_provider_validation::validation;


pub mod editor_merge;

pub use app_provider_core::snippets;
#[cfg(all(test, target_os = "macos"))]
mod snippets_tests;

pub use app_auth::providers::claude;

#[cfg(target_os = "macos")]
pub use app_auth::providers::claude_login;

pub use app_provider_projection::codex;

#[cfg(target_os = "macos")]
pub use app_provider_login::codex_login;
#[cfg(target_os = "macos")]
pub(crate) mod codex_login_material {
    pub(crate) use app_auth_codex::identity::extract_codex_auth_user_identity;
}

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
pub use app_provider_native::providers::hermes;
#[cfg(target_os = "macos")]
pub use app_provider_native::providers::openclaw;
#[cfg(target_os = "macos")]
pub use app_provider_native::providers::opencode;

#[cfg(target_os = "macos")]

#[cfg(target_os = "macos")]
pub use app_provider_native::providers::pi;

#[cfg(target_os = "macos")]
pub use app_provider_native::providers::omp;

#[cfg(target_os = "macos")]
pub use app_provider_native::providers::mcode;

#[cfg(target_os = "macos")]
pub use app_auth::providers::kimi;

#[cfg(target_os = "macos")]
pub use app_auth::providers::dsh;

#[cfg(target_os = "macos")]
pub mod agy;


pub mod import;
#[cfg(test)]
mod import_tests;

#[cfg(target_os = "macos")]
pub use app_auth::providers::mode_state;

#[cfg(target_os = "macos")]
pub mod mode_apply;
#[cfg(all(test, target_os = "macos"))]
mod mode_apply_tests;

pub use app_provider_environment::{env_clean, env_diagnose, env_restore};

pub use app_auth::providers::operations;

pub use app_provider_core::basic;

pub use app_provider_validation::request_options;

#[cfg(target_os = "macos")]
mod mode_apply_publication;

#[cfg(all(test, target_os = "macos"))]
mod mode_apply_publication_tests;

#[cfg(all(test, target_os = "macos"))]
mod hermes_catalog_tests;

#[cfg(all(test, target_os = "macos"))]
mod openclaw_catalog_tests;

#[cfg(test)]
mod facade_validation_tests;
