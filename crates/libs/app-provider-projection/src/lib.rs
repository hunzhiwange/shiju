//! Pure provider native projections. No live commits or credential lookup.
pub mod codex;
pub mod projection;
#[cfg(target_os = "macos")]
pub mod opencode_schema;
#[cfg(test)]
mod codex_tests;
