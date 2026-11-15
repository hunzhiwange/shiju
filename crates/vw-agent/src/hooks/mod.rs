//! Host hook runtime composition and public library exports.
pub use agent_hooks::{builtin, HookHandler, HookResult, HookRunner};
mod runtime;
pub use runtime::{HookRuntime, HookRuntimeCandidate, build_hook_runtime, new_hook_runner};
pub(crate) use runtime::{authorize_model_request, execution_hook_runner};
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
