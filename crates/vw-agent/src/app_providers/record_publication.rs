//! Provider record reservation lives in the existing provider namespace.
use super::{GatewaySettingsStore, NAMESPACE};
use crate::gateway::settings_store::GatewaySetting;
use anyhow::Result;
#[cfg(target_os = "macos")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use vw_agent_apps::providers::live::{record_publication::*, write_guard::WriteError};
use vw_api_types::id::TenantScope;

#[cfg(target_os = "macos")]
type ResyncContext = std::sync::Arc<crate::gateway::aggregation_resync::RecordPublicationContext>;
#[derive(Clone)]
pub(super) struct ProviderRecordStore(
    pub(super) GatewaySettingsStore,
    #[cfg(target_os = "macos")] pub(super) Option<ResyncContext>,
);
impl std::ops::Deref for ProviderRecordStore {
    type Target = GatewaySettingsStore;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reservation {
    shiju_provider_publication: RecordReservation,
}
#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordReservation {
    operation_id: String,
    host: vw_agent_apps::providers::live::commit::ExecutionHostId,
    project: vw_api_types::id::ProjectScope,
    app: vw_api_types::agent_apps::AppId,
    record: ProviderRecordDelta,
}
impl ProviderRecordStore {
    pub(super) fn new(raw: GatewaySettingsStore) -> Self {
        Self(
            raw,
            #[cfg(target_os = "macos")]
            None,
        )
    }
    pub(super) fn compare_exchange(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        key: &str,
        expected: &str,
        after: &str,
    ) -> Result<bool> {
        #[cfg(target_os = "macos")]
        if namespace == NAMESPACE
            && let Some(context) = &self.1
        {
            if tenant != context.project.tenant_scope() {
                anyhow::bail!("provider publication scope mismatch");
            }
            return context.replace(key, expected, after).map_err(|status| {
                if let Ok(mut failure) = context.failure.lock() {
                    *failure = Some(status);
                }
                anyhow::anyhow!("provider publication failed")
            });
        }
        if namespace == NAMESPACE
            && serde_json::from_str::<serde_json::Value>(expected)?
                .get("shijuPublicationBinding")
                .is_some()
        {
            anyhow::bail!("provider requires project publication");
        }
        self.0.compare_exchange(tenant, namespace, key, expected, after)
    }
    pub(super) fn committed(raw: String) -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            if value.get("shiju_provider_publication").is_some() {
                let reserved: Reservation = serde_json::from_value(value)?;
                return Ok(reserved.shiju_provider_publication.record.before);
            }
        }
        Ok(raw)
    }
    pub(super) fn get(
        &self,
        tenant: &TenantScope,
        namespace: &str,
        key: &str,
    ) -> Result<Option<String>> {
        let raw = self.0.get(tenant, namespace, key)?;
        if namespace == NAMESPACE { raw.map(Self::committed).transpose() } else { Ok(raw) }
    }
    pub(super) fn list(
        &self,
        tenant: &TenantScope,
        namespace: &str,
    ) -> Result<Vec<GatewaySetting>> {
        let mut rows = self.0.list(tenant, namespace)?;
        if namespace == NAMESPACE {
            for row in &mut rows {
                row.payload_json = Self::committed(std::mem::take(&mut row.payload_json))?;
            }
        }
        Ok(rows)
    }
    pub(super) fn delete(&self, tenant: &TenantScope, namespace: &str, key: &str) -> Result<bool> {
        if namespace != NAMESPACE {
            return self.0.delete(tenant, namespace, key);
        }
        let Some(raw) = self.get(tenant, namespace, key)? else { return Ok(false) };
        if serde_json::from_str::<serde_json::Value>(&raw)?.get("shijuPublicationBinding").is_some()
        {
            return Ok(false);
        }
        self.0.compare_delete(tenant, namespace, key, &raw)
    }
}
#[cfg(target_os = "macos")]
fn reservation(
    change: &ProviderRecordChange,
    id: &str,
    before: &str,
    after: &str,
) -> Result<String, WriteError> {
    serde_json::to_string(&Reservation {
        shiju_provider_publication: RecordReservation {
            operation_id: change.operation_id.clone(),
            host: change.host.clone(),
            project: change.project.clone(),
            app: change.app,
            record: ProviderRecordDelta {
                provider_id: id.into(),
                before: before.into(),
                after: after.into(),
            },
        },
    })
    .map_err(|_| WriteError::InvalidResource)
}
#[cfg(target_os = "macos")]
impl ProviderRecordParticipant for super::AppProviderStore {
    fn reserve(&self, change: &ProviderRecordChange) -> Result<(), WriteError> {
        let tenant = change.project.tenant_scope();
        for (id, before, after) in change.records() {
            let key = Self::key(change.app, id).map_err(|_| WriteError::InvalidResource)?;
            let reserved = reservation(change, id, before, after)?;
            let current =
                self.settings.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)?;
            if current.as_deref() == Some(&reserved) {
                continue;
            }
            if current.as_deref() != Some(before)
                || !self
                    .settings
                    .0
                    .compare_exchange(tenant, NAMESPACE, &key, before, &reserved)
                    .map_err(|_| WriteError::Io)?
            {
                return Err(WriteError::Conflict);
            }
        }
        Ok(())
    }
    fn finish(&self, change: &ProviderRecordChange, published: bool) -> Result<(), WriteError> {
        let tenant = change.project.tenant_scope();
        for (id, before, after) in change.records() {
            let key = Self::key(change.app, id).map_err(|_| WriteError::InvalidResource)?;
            let reserved = reservation(change, id, before, after)?;
            let current =
                self.settings.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)?;
            let target = if published { after } else { before };
            if current.as_deref() == Some(target) {
                continue;
            }
            if current.as_deref() != Some(&reserved) {
                if published { return Err(WriteError::Conflict) } else { continue }
            }
            if !self
                .settings
                .0
                .compare_exchange(tenant, NAMESPACE, &key, &reserved, target)
                .map_err(|_| WriteError::Io)?
            {
                return Err(WriteError::Conflict);
            }
        }
        Ok(())
    }
}

impl super::AppProviderStore {
    pub(crate) fn publication_record(
        &self,
        tenant: &TenantScope,
        app: vw_api_types::agent_apps::AppId,
        id: &str,
    ) -> Result<String, String> {
        self.settings
            .get(tenant, super::NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "provider storage unavailable")?
            .ok_or_else(|| "application provider not found".into())
    }
}
