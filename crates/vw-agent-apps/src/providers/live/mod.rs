//! Internal live configuration primitives; callers authorize resources and own transactions.
pub mod patch_dotenv;
#[cfg(test)]
mod patch_dotenv_tests;
pub mod patch_json;
#[cfg(test)]
mod patch_json_tests;
pub mod patch_toml;
#[cfg(test)]
mod patch_toml_tests;
pub mod patch_yaml;
#[cfg(test)]
mod patch_yaml_tests;

#[cfg(target_os = "macos")]
pub mod write_guard;
#[cfg(all(test, target_os = "macos"))]
mod write_guard_tests;

#[cfg(target_os = "macos")]
pub mod stage;
#[cfg(all(test, target_os = "macos"))]
mod stage_tests;

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
