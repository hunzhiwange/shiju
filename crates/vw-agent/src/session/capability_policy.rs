//! Native defaults resolved from trusted host configuration.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use session_engine::capability_policy::{
    CapabilityPolicyResolution, normalize_capability_id, resolve_skill_capabilities,
    resolve_skill_defaults,
};

pub(crate) use session_engine::capability_policy::NativeAgentDefaults;
pub(crate) async fn native_agent_defaults(config: &crate::config::Config, key: &str, explicit_model: Option<&str>, workspace: &std::path::Path) -> anyhow::Result<Option<NativeAgentDefaults>> {
    session_engine::capability_policy::native_agent_defaults::<super::agent_binding::AdmissionHost>(config, key, explicit_model, workspace).await
}
