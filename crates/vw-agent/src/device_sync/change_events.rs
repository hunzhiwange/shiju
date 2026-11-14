//! Post-commit hints shared by both transports. No payloads or credentials cross this boundary.
use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};
use tokio::sync::watch;
use vw_api_types::id::TenantScope;

type TenantKey = (String, String);
fn key(tenant: &TenantScope) -> TenantKey {
    (tenant.platform_id().as_ref().to_owned(), tenant.company_id().as_ref().to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDomain {
    Providers,
    ProviderEndpoints,
    McpServers,
    Prompts,
    Skills,
    SkillRepos,
    Profiles,
    Settings,
    ProxyConfig,
}

/// Explicit allowlist: usage, host state and model_pricing never become sync hints.
pub fn domain_for_source(source: &str) -> Option<SyncDomain> {
    use SyncDomain::*;
    Some(match source {
        "providers" | "app_providers.records" => Providers,
        "provider_endpoints" | "app_providers.endpoints" => ProviderEndpoints,
        "mcp_servers" | "agent_apps.mcp.catalog.v1" => McpServers,
        s if s.starts_with("agent_apps.mcp.catalog.v1.project.") => McpServers,
        "prompts" | "agent_apps.prompts.v1" => Prompts,
        "skills" | "agent_apps.skills.installed" => Skills,
        "skill_repos" | "agent_apps.skills.repos" => SkillRepos,
        "profiles" | "project_profiles" => Profiles,
        "settings" | "agent_apps_user" => Settings,
        "proxy_config" => ProxyConfig,
        _ => return None,
    })
}

fn channels() -> &'static Mutex<BTreeMap<TenantKey, watch::Sender<u64>>> {
    static CHANNELS: OnceLock<Mutex<BTreeMap<TenantKey, watch::Sender<u64>>>> = OnceLock::new();
    CHANNELS.get_or_init(Default::default)
}

/// Subscribe before reading the initial snapshot. Watch coalesces changes without
/// losing the dirty state when consumers are slow. Each transport subscribes separately.
pub fn subscribe(tenant: &TenantScope) -> watch::Receiver<u64> {
    let mut channels = channels().lock().unwrap_or_else(|e| e.into_inner());
    channels.entry(key(tenant)).or_insert_with(|| watch::channel(0).0).subscribe()
}

thread_local! {
    static SUPPRESSION: RefCell<BTreeMap<TenantKey, usize>> = RefCell::new(BTreeMap::new());
}

/// Synchronous owner publication only: never carry thread-local suppression over
/// an await. Nesting and unwinding restore precisely this tenant's prior state.
pub(crate) fn without_echo<T>(tenant: &TenantScope, publish: impl FnOnce() -> T) -> T {
    struct Guard(TenantKey);
    impl Drop for Guard {
        fn drop(&mut self) {
            SUPPRESSION.with(|depths| {
                let mut depths = depths.borrow_mut();
                if let Some(depth) = depths.get_mut(&self.0) {
                    *depth -= 1;
                    if *depth == 0 {
                        depths.remove(&self.0);
                    }
                }
            });
        }
    }
    let tenant_key = key(tenant);
    SUPPRESSION.with(|depths| *depths.borrow_mut().entry(tenant_key.clone()).or_default() += 1);
    let _guard = Guard(tenant_key);
    publish()
}

/// Domain owners call only after successful commit, never from SQL update hooks
/// or rollback paths. A hint triggers a fresh authorized snapshot, not an upload
/// of the original setting (which may include device-only fields).
pub(crate) fn committed(tenant: &TenantScope, source: &str) {
    if domain_for_source(source).is_none() {
        return;
    }
    let tenant_key = key(tenant);
    if SUPPRESSION.with(|depths| depths.borrow().contains_key(&tenant_key)) {
        return;
    }
    let mut channels = channels().lock().unwrap_or_else(|e| e.into_inner());
    channels
        .entry(tenant_key)
        .or_insert_with(|| watch::channel(0).0)
        .send_modify(|generation| *generation = generation.wrapping_add(1));
}
