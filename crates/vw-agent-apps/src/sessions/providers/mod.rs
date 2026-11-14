//! Native application session adapters.
pub mod claude;

pub mod hermes;
pub mod pi;

pub mod opencode;

pub(crate) mod opencode_blocks;

pub mod opencode_sqlite;

pub mod codex;

pub mod gemini;

pub mod hermes_sqlite;

pub(crate) mod pi_blocks;
pub(crate) mod pi_jsonl;
mod pi_tree;

pub mod grokbuild;

mod codex_messages;
mod codex_items;

pub mod openclaw;

pub mod kimi;
pub(crate) mod kimi_requests;

pub mod mcode;

pub mod omp;

pub mod dsh;

pub mod agy;
