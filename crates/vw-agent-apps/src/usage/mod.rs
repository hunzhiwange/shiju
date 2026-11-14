//! Native usage queries and their account, credential, and capability selection.

#[cfg(not(target_arch = "wasm32"))]
pub mod balances;
pub mod binding;
pub mod cache;
pub mod claude;
pub mod claude_scan;
pub mod codex_delta;
pub mod codex_lineage;
pub mod codex_timing;
pub mod cost;
pub mod cursors;
#[cfg(not(target_arch = "wasm32"))]
pub mod coding;
pub mod detect;
pub mod gemini;
pub mod grokbuild;
pub mod limits;
pub mod logs;
pub mod exchange;
pub mod exchange_store;
pub mod tokens;
pub mod model_names;
pub mod presentation;
pub mod ranges;
pub mod pi;
pub mod pi_children;
pub mod scripts;
pub mod store;
pub mod source_settings;
#[cfg(target_os = "macos")]
pub mod subscriptions;

#[cfg(test)]
mod binding_tests;
#[cfg(all(test, target_os = "macos"))]
mod cache_tests;
#[cfg(test)]
mod detect_tests;

pub mod opencode;

pub mod mcode;

pub mod agy;
mod agy_native;

pub mod kimi;

pub mod dsh;

pub mod retention;

pub mod summary;

pub mod groups;

pub mod series;

pub mod omp;

pub mod pricing;

pub(crate) mod import_store;
