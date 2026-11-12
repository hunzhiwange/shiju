//! Durable delivery evidence for the first prompt of an explicit turn fork.
//! The caller holds lifecycle::try_lock for the entire request, including persistence.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use vw_acp::SessionRecord;

use super::lifecycle::session_identity;
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

const NAMESPACE: &str = "acp.fork-history-delivery";

#[derive(Clone, Serialize, Deserialize)]
struct Delivery {
    record_id: String,
    remote_id: String,
    completed: bool,
}

pub(super) struct ForkReplay {
    store: GatewaySettingsStore,
    tenant: vw_api_types::id::TenantScope,
    key: String,
    delivery: Option<Delivery>,
}

impl ForkReplay {
    pub(super) async fn load(session: &str, directory: &str) -> Result<Option<Self>> {
        if session.is_empty() {
            return Ok(None);
        }
        let Some(project) = crate::project::instance::project() else {
            return Ok(None);
        };
        if crate::session::fork_execution::load_scoped(&project.id, session).await?.is_none() {
            return Ok(None);
        }
        let store =
            GatewaySettingsStore::new(GlobalDatabase::for_home(&crate::global::paths().home)?)?;
        let tenant = crate::session::ui_store::current_tenant_scope();
        let key = session_identity(session, directory);
        let delivery: Option<Delivery> = store
            .get(&tenant, NAMESPACE, &key)?
            .map(|value| serde_json::from_str(&value))
            .transpose()?;
        if delivery.as_ref().is_some_and(|state| !state.completed) {
            bail!(
                "fork history delivery is uncertain after an interrupted prompt; create a new fork to retry safely"
            );
        }
        Ok(Some(Self { store, tenant, key, delivery }))
    }

    pub(super) fn needs_history(&self) -> bool {
        self.delivery.is_none()
    }

    pub(super) fn validate_record(&self, record: &SessionRecord) -> Result<()> {
        if let Some(delivery) = &self.delivery {
            if delivery.record_id != record.vwacp_record_id
                || delivery.remote_id != record.acp_session_id
            {
                bail!(
                    "fork history is anchored to a different ACP session; refusing replay or history loss"
                );
            }
        } else if record.last_prompt_at.is_some() {
            bail!("fork ACP session already contains prompts without history delivery evidence");
        }
        Ok(())
    }

    fn save(&mut self, delivery: Delivery) -> Result<()> {
        self.store.upsert(
            &self.tenant,
            NAMESPACE,
            &self.key,
            &serde_json::to_string(&delivery)?,
        )?;
        self.delivery = Some(delivery);
        Ok(())
    }

    /// Persist uncertainty before issuing any prompt. Cancellation/crash keeps this evidence.
    pub(super) fn begin(&mut self, record: &SessionRecord) -> Result<()> {
        self.validate_record(record)?;
        if self.needs_history() {
            self.save(Delivery {
                record_id: record.vwacp_record_id.clone(),
                remote_id: record.acp_session_id.clone(),
                completed: false,
            })?;
        }
        Ok(())
    }

    /// Only called after a successful prompt and durable remote-session record write.
    pub(super) fn complete(&mut self, record: &SessionRecord) -> Result<()> {
        self.validate_record(record)?;
        if let Some(delivery) = &self.delivery
            && !delivery.completed
        {
            let mut delivery = delivery.clone();
            delivery.completed = true;
            self.save(delivery)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "fork_replay_tests.rs"]
mod tests;
