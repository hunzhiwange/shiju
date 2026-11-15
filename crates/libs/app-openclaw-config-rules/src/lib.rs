//! Pure OpenClaw catalog and defaults rules; callers own authorization and publication.
#![cfg(target_os = "macos")]

pub mod catalog;
pub mod defaults;

#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod defaults_tests;
