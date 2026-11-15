//! Provider catalog rules without host access or native activation.
use app_provider_validation::{validate_identity, validate_profile};
use app_provider_validation::validation;

pub mod basic;
pub mod create;
#[cfg(test)]
mod create_tests;
pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;
pub mod icons;
#[cfg(test)]
mod icons_tests;
pub mod presentation;
#[cfg(test)]
mod presentation_tests;
pub mod snippets;
#[cfg(test)]
mod snippets_tests;
pub mod universal;
#[cfg(test)]
mod universal_tests;
pub mod update;
#[cfg(test)]
mod update_tests;
pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;
pub mod failover;
pub mod universal_sync;
