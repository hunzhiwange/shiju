//! Ordinary activation adapter; native authorization is supplied by the dispatcher.
use super::*;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::{prompts::{activation, targets}, providers::live::{
    commit::ProviderTransactions, write_guard::{lock_app, WriteError},
}};

impl PromptsStore {
    /// Activate or save the active entry with immediate native projection. Host
    /// inputs are executor-owned, never deserialized from the request. The
    /// callback rechecks action/physical-slot ownership before publication.
    /// `edit` is accepted only for the current selection; disabled edits use update.
    #[allow(clippy::too_many_arguments)]
    pub fn activate_ordinary(
        &self, scope: PromptScope<'_>, revision: u64, id: &str,
        edit: Option<PromptEntry>, executor_host: &str, home: &Path,
        override_dir: Option<&str>, environment: &BTreeMap<String, String>,
        transactions: &ProviderTransactions,
        authorize: impl Fn() -> Result<(), WriteError>,
    ) -> Result<PromptCatalog> {
        let key = scope.key()?;
        ensure!(!scope.host.is_empty() && scope.host == executor_host, "unauthorized prompt host");
        authorize().map_err(|_| anyhow::anyhow!("prompt authorization denied"))?;
        let name = activation::filename(scope.app)?;
        let target = targets::resolve(home, scope.app, override_dir, environment)
            .map_err(anyhow::Error::msg)?;
        let directory = target.file.parent().ok_or_else(|| anyhow::anyhow!("invalid prompt target"))?;
        let mut guard = lock_app(scope.app, directory)
            .map_err(|_| anyhow::anyhow!("prompt target lock failed"))?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let mut catalog = Self::decode(previous.as_deref())?;
        ensure!(catalog.revision == revision, "prompt catalog revision conflict");
        if let Some(edit) = &edit {
            validate_entry(edit)?;
            ensure!(edit.id == id && catalog.active_id.as_deref() == Some(id),
                "active edit requires the current selection");
        }
        let live = guard.read(name).map_err(|_| anyhow::anyhow!("prompt file read failed"))?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?
            .as_millis().try_into()?;
        let (entries, content) = activation::prepare(&catalog.entries,
            catalog.active_id.as_deref(), id, edit.as_ref(), live.as_deref(), now)?;
        catalog.entries = entries;
        catalog.active_id = Some(id.to_owned());
        catalog.revision = revision.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("prompt catalog revision exhausted"))?;
        let payload = serde_json::to_string(&catalog)?;
        activation::publish(&mut guard, transactions, scope.project, live.as_deref(), &content,
            || {
                authorize()?;
                let current = self.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)?;
                if current != previous { return Err(WriteError::Conflict); }
                Ok(())
            },
            || {
                let changed = match previous.as_deref() {
                    Some(raw) => self.0.compare_exchange(tenant, NAMESPACE, &key, raw, &payload),
                    None => self.0.insert_if_absent(tenant, NAMESPACE, &key, &payload),
                }.map_err(|_| ())?;
                if !changed { return Err(()); }
                Ok(())
            },
        )?;
        catalog.entries = list_entries(catalog.entries, "");
        Ok(catalog)
    }
}
