//! Internal live configuration primitives; callers authorize resources and own transactions.
pub use app_config_patch::{patch_dotenv, patch_json, patch_toml, patch_yaml};

#[cfg(target_os = "macos")]
pub use app_config_write::write_guard;

#[cfg(target_os = "macos")]
pub use app_config_write::stage;

#[cfg(target_os = "macos")]
pub mod claude_directory;
#[cfg(all(test, target_os = "macos"))]
mod claude_directory_tests;
#[cfg(target_os = "macos")]
pub mod commit;
#[cfg(all(test, target_os = "macos"))]
mod commit_tests;

#[cfg(target_os = "macos")]
pub mod recovery;
#[cfg(all(test, target_os = "macos"))]
mod recovery_tests;

#[cfg(target_os = "macos")]
pub mod concurrency;
#[cfg(all(test, target_os = "macos"))]
mod concurrency_tests;

#[cfg(target_os = "macos")]
pub mod project_binding;
#[cfg(all(test, target_os = "macos"))]
mod project_binding_tests;

#[cfg(target_os = "macos")]
pub mod record_publication;

#[cfg(target_os = "macos")]
pub use app_config_patch::patch_json5;
