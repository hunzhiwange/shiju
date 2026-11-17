//! Host composition for the canonical AI SDK implementation.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use llm_runtime::aisdk::convert::*;

#[cfg(test)]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/convert_tests.rs"]
mod convert_tests;
