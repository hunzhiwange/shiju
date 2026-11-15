//! Lifecycle hook contracts, dispatch, audit and replaceable runtime.
pub mod builtin;
mod runner;
mod runtime;
pub mod traits;
pub use runner::HookRunner;
pub use runtime::{HookRuntime, HookRuntimeCandidate, build_hook_runtime};
pub use traits::{HookHandler, HookResult};

#[derive(Default, Debug)]
pub struct TurnEndOutcome {
    pub stop: bool,
    pub contexts: Vec<String>,
}
impl TurnEndOutcome {
    pub fn merge(&mut self, other: Self) {
        self.stop |= other.stop;
        self.contexts.extend(other.contexts);
        if self.stop {
            self.contexts.clear();
        }
    }
}

#[cfg(test)]
mod tests;

/// Host resource cleanup that must finish before lifecycle notifications.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait HookCleanup: traits::HookHandlerBounds {
    async fn gateway_stop(&self);
    async fn session_end(&self, session_id: &str);
}
