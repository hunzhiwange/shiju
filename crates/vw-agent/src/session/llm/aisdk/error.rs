//! Host composition for the canonical AI SDK implementation.
pub(crate) use llm_runtime::aisdk::error::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/error_tests.rs"]
mod error_tests;
