pub mod model_ids;

#[cfg(test)]
mod model_ids_tests;

pub mod omo_read;

#[cfg(test)]
mod omo_read_tests;

pub mod member_keys;

#[cfg(all(test, target_os = "macos"))]
mod member_keys_tests;

pub mod codex_review;

#[cfg(test)]
mod codex_review_tests;

pub mod codex_classic;

#[cfg(test)]
mod codex_classic_tests;

pub mod member_validation;

#[cfg(test)]
mod member_validation_tests;

#[cfg(target_os = "macos")]
pub mod omo_write;

#[cfg(all(test, target_os = "macos"))]
mod omo_write_tests;

#[cfg(target_os = "macos")]
pub mod omo_slim;

#[cfg(all(test, target_os = "macos"))]
mod omo_slim_tests;

#[cfg(target_os = "macos")]
pub mod membership;

#[cfg(all(test, target_os = "macos"))]
mod membership_tests;

pub mod codex_catalog;

#[cfg(test)]
mod codex_catalog_tests;

pub mod codex_subagent;

#[cfg(test)]
mod codex_subagent_tests;

pub mod claude_catalog;

#[cfg(test)]
mod claude_catalog_tests;

#[cfg(target_os = "macos")]
pub mod resolve;

#[cfg(all(test, target_os = "macos"))]
mod resolve_tests;
