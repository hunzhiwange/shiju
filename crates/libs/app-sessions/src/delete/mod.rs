//! Destructive operations accept only host-authorized registry identities.
pub mod dispatch;
pub use dispatch::delete_session;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod openclaw;
pub mod grokbuild;
pub mod pi;
pub mod omp;
pub mod kimi;
pub mod opencode;
pub mod hermes;

#[cfg(test)]
mod hermes_tests;

pub mod opencode_sqlite;

pub mod hermes_sqlite;
