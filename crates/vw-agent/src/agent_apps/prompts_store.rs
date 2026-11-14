//! Disabled prompt catalog persisted through the shared SQLite/PostgreSQL settings backend.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use vw_agent_apps::providers::validate_identity;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

const NAMESPACE: &str = "agent_apps.prompts.v1";

/// Identities must be supplied by the authorized host dispatcher, never trusted from a payload.
#[derive(Clone, Copy)]
pub struct PromptScope<'a> {
    pub project: &'a ProjectScope,
    pub host: &'a str,
    pub app: AppId,
}

pub use vw_agent_apps::prompts::listing::PromptEntry;
use vw_agent_apps::prompts::listing::list_entries;

/// Revision covers the whole scoped catalog, including deletion of its last entry.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptCatalog {
    pub revision: u64,
    /// Successful first access, including an absent or blank native file.
    #[serde(default)]
    pub bootstrap_complete: bool,
    #[serde(default)]
    pub active_id: Option<String>,
    pub entries: Vec<PromptEntry>,
}

/// Each target reports its own persisted copy or authorization/storage failure.
pub struct PromptCopyResult {
    pub app: AppId,
    pub result: Result<PromptEntry>,
}

#[derive(Clone)]
pub struct PromptsStore(GatewaySettingsStore);

impl PromptsStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        Ok(Self(GatewaySettingsStore::new(database)?))
    }

    pub fn list(&self, scope: PromptScope<'_>) -> Result<PromptCatalog> {
        self.search(scope, "")
    }

    /// Search only the authorized tenant/project/host/application catalog.
    pub fn search(&self, scope: PromptScope<'_>, query: &str) -> Result<PromptCatalog> {
        let key = scope.key()?;
        let mut catalog = Self::decode(self.0.get(scope.project.tenant_scope(), NAMESPACE, &key)?.as_deref())?;
        catalog.entries = list_entries(catalog.entries, query);
        Ok(catalog)
    }

    pub fn get(&self, scope: PromptScope<'_>, id: &str) -> Result<Option<PromptEntry>> {
        validate_identity(id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
        Ok(self.list(scope)?.entries.into_iter().find(|entry| entry.id == id))
    }

    pub fn create(&self, scope: PromptScope<'_>, revision: u64, mut entry: PromptEntry) -> Result<PromptCatalog> {
        validate_entry(&entry)?;
        entry.created_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis().try_into()?);
        self.mutate(scope, revision, |entries| {
            ensure!(!entries.iter().any(|item| item.id == entry.id), "prompt already exists");
            entries.push(entry);
            Ok(())
        })
    }

    /// Create-only share import preserving the orchestration owner's ID/time.
    pub fn create_shared(&self, scope: PromptScope<'_>, revision: u64, entry: PromptEntry) -> Result<PromptCatalog> {
        validate_entry(&entry)?;
        self.mutate(scope, revision, |entries| {
            ensure!(!entries.iter().any(|item| item.id == entry.id), "prompt already exists");
            entries.push(entry);
            Ok(())
        })
    }

    /// Copy the authorized source snapshot into independent disabled catalogs.
    /// Authorization is evaluated for the source and every target on this host.
    /// Partial success is intentional; a failed target never rolls back siblings.
    /// The callback must check resource permissions, not just app availability.
    pub fn copy_to_apps(
        &self,
        source: PromptScope<'_>,
        id: &str,
        targets: &[AppId],
        mut authorize: impl FnMut(PromptScope<'_>) -> Result<()>,
    ) -> Result<Vec<PromptCopyResult>> {
        authorize(source)?;
        vw_api_types::agent_apps::PromptResource::Application { app_id: source.app }
            .validate_application_access()
            .map_err(|_| anyhow::anyhow!("unsupported prompt copy source"))?;
        let original = self.get(source, id)?
            .ok_or_else(|| anyhow::anyhow!("prompt not found"))?;
        let mut results = Vec::with_capacity(targets.len());
        for (index, &app) in targets.iter().enumerate() {
            let result = (|| {
                ensure!(app != source.app, "prompt copy target must differ from source");
                ensure!(!targets[..index].contains(&app), "duplicate prompt copy target");
                let scope = PromptScope { app, ..source };
                authorize(scope)?;
                let id = vw_agent_apps::prompts::copy::new_copy_id(app, &original.content)
                    .map_err(anyhow::Error::msg)?;
                let entry = PromptEntry { id, ..original.clone() };
                let catalog = self.list(scope)?;
                // Existing same-name entries are preserved. CAS rejects concurrent
                // catalog changes instead of overwriting them or retrying invisibly.
                let saved = self.create(scope, catalog.revision, entry.clone())?;
                saved.entries.into_iter().find(|saved| saved.id == entry.id)
                    .ok_or_else(|| anyhow::anyhow!("created prompt copy not found"))
            })();
            results.push(PromptCopyResult { app, result });
        }
        Ok(results)
    }

    /// Explicit current-file import. The dispatcher supplies authenticated host
    /// paths/environment after tenant and action authorization. Reading and
    /// catalog creation never invoke activation or write the native file.
    #[cfg(target_os = "macos")]
    pub fn import_current_file(
        &self,
        scope: PromptScope<'_>,
        revision: u64,
        executor_host: &str,
        home: &std::path::Path,
        override_dir: Option<&str>,
        environment: &std::collections::BTreeMap<String, String>,
    ) -> Result<PromptCatalog> {
        // Validate scope before host IO, including on an empty catalog.
        scope.key()?;
        let preview = vw_agent_apps::prompts::live_file::read(
            scope.host, executor_host, home, scope.app, override_dir, environment,
        )?;
        let imported = vw_agent_apps::prompts::import::from_snapshot(preview.snapshot)?;
        self.create_import(scope, revision, imported)
    }

    /// Uploads carry selected bytes, never a path on the execution host.
    /// Authorization of the scope and import action belongs to the dispatcher.
    pub fn import_uploaded_content(
        &self,
        scope: PromptScope<'_>,
        revision: u64,
        name: String,
        content: Vec<u8>,
        description: Option<String>,
    ) -> Result<PromptCatalog> {
        scope.key()?;
        let imported = vw_agent_apps::prompts::import::from_upload(
            scope.app, name, content, description,
        )?;
        self.create_import(scope, revision, imported)
    }

    fn create_import(
        &self,
        scope: PromptScope<'_>,
        revision: u64,
        imported: vw_agent_apps::prompts::import::DisabledImport,
    ) -> Result<PromptCatalog> {
        self.create(scope, revision, PromptEntry {
            id: imported.id,
            name: imported.name,
            content: imported.content,
            description: imported.description,
            created_at: None,
        })
    }

    pub fn update(&self, scope: PromptScope<'_>, revision: u64, entry: PromptEntry) -> Result<PromptCatalog> {
        validate_entry(&entry)?;
        ensure!(self.list(scope)?.active_id.as_deref() != Some(entry.id.as_str()),
            "active prompt edits require activate_ordinary");
        self.mutate(scope, revision, |entries| {
            let existing = entries.iter_mut().find(|item| item.id == entry.id)
                .ok_or_else(|| anyhow::anyhow!("prompt not found"))?;
            let created_at = existing.created_at;
            *existing = entry;
            existing.created_at = created_at;
            Ok(())
        })
    }

    pub fn delete(&self, scope: PromptScope<'_>, revision: u64, id: &str) -> Result<PromptCatalog> {
        validate_identity(id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
        ensure!(self.list(scope)?.active_id.as_deref() != Some(id), "deactivate prompt before deletion");
        self.mutate(scope, revision, |entries| {
            let index = entries.iter().position(|entry| entry.id == id)
                .ok_or_else(|| anyhow::anyhow!("prompt not found"))?;
            entries.remove(index);
            Ok(())
        })
    }

    fn mutate(&self, scope: PromptScope<'_>, revision: u64,
        operation: impl FnOnce(&mut Vec<PromptEntry>) -> Result<()>,
    ) -> Result<PromptCatalog> {
        let key = scope.key()?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let mut catalog = Self::decode(previous.as_deref())?;
        ensure!(catalog.revision == revision, "prompt catalog revision conflict");
        operation(&mut catalog.entries)?;
        catalog.revision = revision.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("prompt catalog revision exhausted"))?;
        let payload = serde_json::to_string(&catalog)?;
        let changed = match previous {
            Some(previous) => self.0.compare_exchange(tenant, NAMESPACE, &key, &previous, &payload)?,
            None => self.0.insert_if_absent(tenant, NAMESPACE, &key, &payload)?,
        };
        ensure!(changed, "prompt catalog revision conflict");
        catalog.entries = list_entries(catalog.entries, "");
        Ok(catalog)
    }

    fn decode(raw: Option<&str>) -> Result<PromptCatalog> {
        let Some(raw) = raw else { return Ok(PromptCatalog::default()); };
        // Do not include raw prompt text in errors or silently overwrite corrupt data.
        let catalog: PromptCatalog = serde_json::from_str(raw)
            .map_err(|_| anyhow::anyhow!("invalid stored prompt catalog"))?;
        for (index, entry) in catalog.entries.iter().enumerate() {
            validate_entry(&entry)?;
            ensure!(!catalog.entries[..index].iter().any(|other| other.id == entry.id),
                "duplicate stored prompt identity");
        }
        if let Some(active) = &catalog.active_id {
            ensure!(catalog.entries.iter().any(|entry| &entry.id == active),
                "invalid stored prompt selection");
        }
        Ok(catalog)
    }
}

fn validate_entry(entry: &PromptEntry) -> Result<()> {
        validate_identity(&entry.id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
        ensure!(!entry.name.trim().is_empty(), "prompt name is empty");
        Ok(())
}

impl PromptScope<'_> {
    fn key(self) -> Result<String> {
        validate_identity(self.host).map_err(|_| anyhow::anyhow!("invalid prompt host"))?;
        if self.app == AppId::ClaudeDesktop {
            bail!("Claude Desktop does not support native prompts");
        }
        // Tuple encoding prevents delimiter collisions. Tenant isolation is enforced by the backend.
        Ok(serde_json::to_string(&(self.project.project_id(), self.host, self.app))?)
    }
}

#[cfg(target_os = "macos")]
#[path = "prompt_activation_store.rs"]
mod activation_store;

#[cfg(target_os = "macos")]
#[path = "prompt_deactivation_store.rs"]
mod deactivation_store;

#[cfg(target_os = "macos")]
#[path = "prompts_bootstrap.rs"]
mod bootstrap;

#[cfg(target_os = "macos")]
pub use deactivation_store::OrdinaryPromptReconciliation;

#[cfg(target_os = "macos")]
#[path = "prompt_sync_store.rs"]
mod sync_store;

#[cfg(target_os = "macos")]
#[path = "prompt_toggle_store.rs"]
mod toggle_store;

#[cfg(target_os = "macos")]
#[path = "profile_prompt_store.rs"]
mod profile_prompt_store;

#[cfg(target_os = "macos")]
#[path = "prompt_data_migration.rs"]
mod data_migration;
#[cfg(target_os = "macos")]
pub use data_migration::{PromptMigration, PromptMigrationCommit};
#[cfg(all(test, target_os = "macos"))]
#[path = "prompt_data_migration_tests.rs"]
mod data_migration_tests;
