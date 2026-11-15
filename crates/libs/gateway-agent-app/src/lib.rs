//! Authorized native application prompt routes.
#![cfg(not(target_arch = "wasm32"))]
pub mod agent_app_prompt_files;
pub mod agent_app_prompt_shared_import;
#[cfg(target_os = "macos")]
pub mod agent_app_prompt_sync;
pub mod agent_app_prompt_templates;
pub mod agent_app_prompts_activation;
pub mod agent_app_prompts_read;
pub mod agent_app_prompts_write;
