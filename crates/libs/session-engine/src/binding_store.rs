//! Persist immutable bindings and permission choices in tenant/project scope.
use anyhow::{Result, bail};
use std::path::PathBuf;
use vw_api_transport::TenantScope;
use crate::agent_binding::AgentBinding;

fn component(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("session binding scope must be non-empty");
    }
    Ok(value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn storage_key(project: &str, session: &str) -> Result<String> {
    Ok(format!("{}:{}", component(project)?, component(session)?))
}

fn store(home: &std::path::Path) -> Result<app_settings_store::settings_store::GatewaySettingsStore> {
    let database = agent_database::GlobalDatabase::for_home(home)?;
    app_settings_store::settings_store::GatewaySettingsStore::new(database)
}

pub async fn load_scoped(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<Option<AgentBinding>> {
    let key = storage_key(project, session)?;
    agent_database::run_isolated(move || {
        store(&home)?
            .get(&tenant, "session.agent-binding", &key)?
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    })
    .await?
}

pub async fn save_scoped(
    home: PathBuf, tenant: TenantScope,
    project: &str,
    session: &str,
    binding: &AgentBinding,
) -> Result<()> {
    let key = storage_key(project, session)?;
    let payload = serde_json::to_string(binding)?;
    agent_database::run_isolated(move || {
        store(&home)?.upsert(&tenant, "session.agent-binding", &key, &payload)
    })
    .await?
}

/// Immutable agent binding and the persisted session permission choice.
pub struct SessionSettings {
    pub binding: Option<AgentBinding>,
    pub full_access: Option<bool>,
}

impl SessionSettings {
    pub async fn load(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<Self> {
        let key = storage_key(project, session)?;
        agent_database::run_isolated(move || {
            let store = store(&home)?;
            Ok(Self {
                binding: store
                    .get(&tenant, "session.agent-binding", &key)?
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?,
                full_access: store
                    .get(&tenant, "session.full-access", &key)?
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?,
            })
        })
        .await?
    }

    pub async fn save(&self, home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<()> {
        let key = storage_key(project, session)?;
        let binding = self.binding.clone();
        let full_access = self.full_access;
        agent_database::run_isolated(move || {
            let store = store(&home)?;
            if let Some(binding) = binding {
                store.upsert(
                    &tenant,
                    "session.agent-binding",
                    &key,
                    &serde_json::to_string(&binding)?,
                )?;
            }
            if let Some(full_access) = full_access {
                store.upsert(
                    &tenant,
                    "session.full-access",
                    &key,
                    &serde_json::to_string(&full_access)?,
                )?;
            }
            Ok(())
        })
        .await?
    }

    pub async fn remove(&self, home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<()> {
        let key = storage_key(project, session)?;
        let has_binding = self.binding.is_some();
        let has_full_access = self.full_access.is_some();
        agent_database::run_isolated(move || {
            let store = store(&home)?;
            if has_binding {
                store.delete(&tenant, "session.agent-binding", &key)?;
            }
            if has_full_access {
                store.delete(&tenant, "session.full-access", &key)?;
            }
            Ok(())
        })
        .await?
    }
}

/// Read a persisted permission choice for automation without granting or storing a default.
pub async fn confirmed_full_access(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<bool> {
    let key = storage_key(project, session)?;
    agent_database::run_isolated(move || {
        Ok(store(&home)?
            .get(&tenant, "session.full-access", &key)?
            .as_deref()
            .map(serde_json::from_str::<bool>)
            .transpose()?
            .unwrap_or(false))
    })
    .await?
}

// Persist the session choice separately from the immutable agent defaults snapshot.
static FULL_ACCESS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn resolve_full_access_scoped(
    home: PathBuf, tenant: TenantScope,
    project: &str,
    session: &str,
    explicit: Option<bool>,
    initial: bool,
) -> Result<bool> {
    let _guard = FULL_ACCESS.lock().await;
    let key = storage_key(project, session)?;
    agent_database::run_isolated(move || {
        let store = store(&home)?;
        let previous = store.get(&tenant, "session.full-access", &key)?;
        let enabled = match explicit {
            Some(enabled) => enabled,
            None => previous
                .as_deref()
                .map(serde_json::from_str::<bool>)
                .transpose()?
                .unwrap_or(initial),
        };
        store.upsert(&tenant, "session.full-access", &key, &serde_json::to_string(&enabled)?)?;
        Ok(enabled)
    })
    .await?
}
