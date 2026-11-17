#[cfg(any(test, target_arch = "wasm32"))]
pub use llm_runtime::logging::LOGGER;
