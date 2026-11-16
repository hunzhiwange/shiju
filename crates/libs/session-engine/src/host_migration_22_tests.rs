//! Processor domain assertions, registered by their owning crate.
mod prompting_tests;
mod helpers_tests;
mod llm_runner_tests;
mod processor_tests;
mod artifacts_tests;
mod planning_tests;
mod prefetch_tests;
mod scripted_stream_tests;
mod child_finalization_tests;
#[cfg(not(target_arch = "wasm32"))]
mod artifact_persistence_tests;
mod todo_storage_tests;
