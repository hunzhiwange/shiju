//! Host composition for the canonical AI SDK implementation.
use model_types::Model;
use serde_json::{Value, json};
use model_reasoning::reasoning_levels;
pub(crate) use llm_runtime::aisdk::reasoning::*;

#[cfg(test)]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/reasoning_tests.rs"]
mod tests;
