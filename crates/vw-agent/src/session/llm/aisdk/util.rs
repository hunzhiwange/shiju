//! Host composition for the canonical AI SDK implementation.
pub(crate) use llm_runtime::aisdk::util::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/util_tests.rs"]
mod util_tests;
