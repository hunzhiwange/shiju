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

/// No enabled field: this store cannot activate prompts or grant native file ownership.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptEntry {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
}

/// Revision covers the whole scoped catalog, including deletion of its last entry.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptCatalog {
    pub revision: u64,
    pub entries: Vec<PromptEntry>,
}

#[derive(Clone)]
pub struct PromptsStore(GatewaySettingsStore);

impl PromptsStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        Ok(Self(GatewaySettingsStore::new(database)?))
    }

    pub fn list(&self, scope: PromptScope<'_>) -> Result<PromptCatalog> {
        let key = scope.key()?;
        Self::decode(self.0.get(scope.project.tenant_scope(), NAMESPACE, &key)?.as_deref())
    }

    pub fn get(&self, scope: PromptScope<'_>, id: &str) -> Result<Option<PromptEntry>> {
        validate_identity(id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
        Ok(self.list(scope)?.entries.into_iter().find(|entry| entry.id == id))
    }

    pub fn create(&self, scope: PromptScope<'_>, revision: u64, entry: PromptEntry) -> Result<PromptCatalog> {
        entry.validate()?;
        self.mutate(scope, revision, |entries| {
            ensure!(!entries.iter().any(|item| item.id == entry.id), "prompt already exists");
            entries.push(entry);
            Ok(())
        })
    }

    pub fn update(&self, scope: PromptScope<'_>, revision: u64, entry: PromptEntry) -> Result<PromptCatalog> {
        entry.validate()?;
        self.mutate(scope, revision, |entries| {
            let existing = entries.iter_mut().find(|item| item.id == entry.id)
                .ok_or_else(|| anyhow::anyhow!("prompt not found"))?;
            *existing = entry;
            Ok(())
        })
    }

    pub fn delete(&self, scope: PromptScope<'_>, revision: u64, id: &str) -> Result<PromptCatalog> {
        validate_identity(id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
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
        Ok(catalog)
    }

    fn decode(raw: Option<&str>) -> Result<PromptCatalog> {
        let Some(raw) = raw else { return Ok(PromptCatalog::default()); };
        // Do not include raw prompt text in errors or silently overwrite corrupt data.
        let catalog: PromptCatalog = serde_json::from_str(raw)
            .map_err(|_| anyhow::anyhow!("invalid stored prompt catalog"))?;
        for (index, entry) in catalog.entries.iter().enumerate() {
            entry.validate()?;
            ensure!(!catalog.entries[..index].iter().any(|other| other.id == entry.id),
                "duplicate stored prompt identity");
        }
        Ok(catalog)
    }
}

impl PromptEntry {
    fn validate(&self) -> Result<()> {
        validate_identity(&self.id).map_err(|_| anyhow::anyhow!("invalid prompt identity"))?;
        ensure!(!self.name.trim().is_empty(), "prompt name is empty");
        Ok(())
    }
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
