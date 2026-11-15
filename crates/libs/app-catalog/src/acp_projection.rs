//! Read-only views of the existing ACP domain; registration is not availability.
use super::catalog::{CatalogApp, catalog};
use std::collections::BTreeMap;
use vw_acp::AgentCommandSpec;
use vw_api_types::agent_apps::{AppId, AppRuntimeKind};

/// Preserve all structured ACP registrations, including agents outside the native catalog.
/// Commands may be local fallbacks; use the review APIs before claiming reviewed support.
pub fn acp_registry() -> BTreeMap<String, AgentCommandSpec> {
    vw_acp::built_in_agent_specs()
}

/// Preserve legacy keys without interpreting them as additional applications.
pub fn legacy_registry() -> BTreeMap<String, String> {
    vw_acp::built_in_agent_registry()
}

/// Add ACP forms to matching native applications without replacing either registry.
pub fn catalog_with_acp() -> Vec<CatalogApp> {
    let registry = acp_registry();
    let mut apps = catalog();
    for app in &mut apps {
        let key = match app.metadata.id {
            AppId::Claude => "claude",
            AppId::Codex => "codex",
            AppId::Gemini => "gemini",
            AppId::OpenCode => "opencode",
            AppId::OpenClaw => "openclaw",
            AppId::Hermes => "hermes",
            AppId::Pi => "pi",
            AppId::Kimi => "kimi",
            _ => continue,
        };
        if registry.contains_key(key) {
            app.metadata.runtime_kinds.push(AppRuntimeKind::AcpAdapter);
        }
    }
    apps
}

fn require_registered(agent: &str) -> Result<(), String> {
    if acp_registry().contains_key(agent) {
        Ok(())
    } else {
        Err("Unknown structured ACP registration".into())
    }
}

/// Keep missing artifacts, checksum/version failures and absent review contracts distinct.
/// `Ok(None)` means no review contract, never that an adapter is installed or approved.
pub fn reviewed_command(agent: &str) -> Result<Option<(String, Vec<String>)>, String> {
    require_registered(agent)?;
    vw_acp::reviewed_agent_command(agent)
}

/// Delegate executable checks to the owner of the pinned review contracts.
pub fn verify_reviewed_executable(agent: &str, command: &str) -> Result<(), String> {
    require_registered(agent)?;
    vw_acp::verify_reviewed_agent_executable(agent, command)
}
