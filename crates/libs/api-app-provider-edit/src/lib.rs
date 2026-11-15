//! Provider catalog and editing contracts.
pub mod catalog;
#[cfg(test)]
mod catalog_tests;
pub mod delete;
#[cfg(test)]
mod delete_tests;
pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;
pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;
pub mod endpoints;
#[cfg(test)]
mod endpoints_tests;
pub mod icons;
#[cfg(test)]
mod icons_tests;
pub mod import;
#[cfg(test)]
mod import_tests;
pub mod presentation;
#[cfg(test)]
mod presentation_tests;
pub mod preview;
#[cfg(test)]
mod preview_tests;
pub mod request_options;
pub mod snippets;
#[cfg(test)]
mod snippets_tests;
pub mod universal;
#[cfg(test)]
mod universal_tests;
pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;
