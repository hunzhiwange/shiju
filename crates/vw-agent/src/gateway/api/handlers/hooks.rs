//! Host composition for hook routes and native runtime operations.
pub(crate) fn router() -> axum::Router<crate::gateway::AppState> {
    gateway_hooks::router()
}

impl gateway_hooks::HookRuntime for crate::hooks::HookRuntime {
    fn build_candidate(&self, config: &crate::config::Config) -> anyhow::Result<agent_hooks::HookRuntimeCandidate> {
        self.build_candidate(config)
    }
    fn replace(&self, candidate: agent_hooks::HookRuntimeCandidate) {
        self.replace(candidate);
    }
    fn audit_store(&self) -> agent_hooks::builtin::CommandAuditStore {
        self.audit_store()
    }
}

// Integration fixtures retain the real host runtime, including native cleanup.
#[cfg(test)]
mod hooks_tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../libs/gateway-hooks/src/hooks_tests.rs"));
}
