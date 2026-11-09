//! Non-secret selection and recovery anchors, scoped to tenant, directory and session.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Weak},
};
use vw_api_types::{acp_model_source::AcpSessionModelSelection, id::TenantScope};

static LOCKS: LazyLock<parking_lot::Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));
const NAMESPACE: &str = "acp.session.selection";

pub(super) fn session_identity(session: &str, directory: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(
        serde_json::to_vec(&serde_json::json!({
            "tenant": crate::session::ui_store::current_tenant_scope(),
            "session": session, "directory": directory,
        }))
        .expect("serializable ACP identity"),
    ))
}

/// One lock covers preparation, replacement, prompts and controls, even across cache keys.
pub(super) fn try_lock(
    session: &str,
    directory: &str,
) -> Result<tokio::sync::OwnedMutexGuard<()>, String> {
    let mut locks = LOCKS.lock();
    locks.retain(|_, value| value.strong_count() > 0);
    let lock = locks.entry(session_identity(session, directory)).or_default();
    let mutex = lock.upgrade().unwrap_or_else(|| {
        let mutex = Arc::new(tokio::sync::Mutex::new(()));
        *lock = Arc::downgrade(&mutex);
        mutex
    });
    mutex
        .try_lock_owned()
        .map_err(|_| "ACP session is busy; retry after the current response".into())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub agent: String,
    pub model: AcpSessionModelSelection,
    pub generation: u64,
    pub record_id: String,
}

impl Selection {
    pub fn has_started(&self) -> bool {
        !self.record_id.is_empty()
    }
}

pub(crate) fn locked_model(
    session: &str,
    directory: &str,
) -> Result<Option<AcpSessionModelSelection>, String> {
    Ok(Store::open(session, directory)
        .and_then(|store| store.read())
        .map_err(|error| error.to_string())?
        .filter(Selection::has_started)
        .map(|selection| selection.model))
}

pub(super) struct Store {
    store: GatewaySettingsStore,
    tenant: TenantScope,
    key: String,
}
impl Store {
    pub fn open(session: &str, directory: &str) -> anyhow::Result<Self> {
        Ok(Self {
            store: GatewaySettingsStore::new(GlobalDatabase::for_home(
                &crate::global::paths().home,
            )?)?,
            tenant: crate::session::ui_store::current_tenant_scope(),
            key: session_identity(session, directory),
        })
    }
    pub fn read(&self) -> anyhow::Result<Option<Selection>> {
        self.store
            .get(&self.tenant, NAMESPACE, &self.key)?
            .map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }
    pub fn commit(&self, selection: &Selection) -> anyhow::Result<()> {
        self.store.upsert(&self.tenant, NAMESPACE, &self.key, &serde_json::to_string(selection)?)
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
