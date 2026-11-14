//! First authorized access imports at most once in the scoped catalog transaction.
use super::*;
use anyhow::Context;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::prompts::{import, live_file};
use vw_api_types::agent_apps::PromptResource;

impl PromptsStore {
    /// Authorized listing entry point for the host dispatcher. Unlike `list`,
    /// this may initialize storage. Authorization must cover catalog mutation
    /// and reading the fixed native file; paths/environment are executor-owned.
    /// Pi selection remains derived from its native file, never persisted here.
    #[allow(clippy::too_many_arguments)]
    pub fn list_on_first_access(
        &self,
        scope: PromptScope<'_>,
        executor_host: &str,
        home: &Path,
        override_dir: Option<&str>,
        environment: &BTreeMap<String, String>,
        authorize: impl Fn() -> Result<()>,
    ) -> Result<PromptCatalog> {
        self.bootstrap_prompts(scope, executor_host, home, override_dir, environment, authorize)
            .context("prompt initialization failed")
    }

    #[allow(clippy::too_many_arguments)]
    fn bootstrap_prompts(
        &self,
        scope: PromptScope<'_>,
        executor_host: &str,
        home: &Path,
        override_dir: Option<&str>,
        environment: &BTreeMap<String, String>,
        authorize: impl Fn() -> Result<()>,
    ) -> Result<PromptCatalog> {
        let key = scope.key()?;
        ensure!(scope.host == executor_host, "unauthorized prompt host");
        PromptResource::Application { app_id: scope.app }.validate_application_access()
            .map_err(|_| anyhow::anyhow!("unsupported prompt bootstrap target"))?;
        authorize()?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let mut catalog = Self::decode(previous.as_deref())?;
        if catalog.bootstrap_complete {
            catalog.entries = list_entries(catalog.entries, "");
            return Ok(catalog);
        }
        // Existing catalogs are sealed without consulting or modifying the file.
        // Deleting their last entry later must not resurrect native content.
        if catalog.entries.is_empty() {
            let preview = live_file::read(
                scope.host, executor_host, home, scope.app, override_dir, environment,
            )?;
            if preview.snapshot.content.as_ref().is_some_and(|text| !text.trim().is_empty()) {
                let imported = import::from_snapshot(preview.snapshot)?;
                let created_at = Some(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?.as_millis().try_into()?);
                if scope.app != AppId::Pi {
                    catalog.active_id = Some(imported.id.clone());
                }
                catalog.entries.push(PromptEntry {
                    id: imported.id,
                    name: imported.name,
                    content: imported.content,
                    description: imported.description,
                    created_at,
                });
            }
        }
        catalog.bootstrap_complete = true;
        catalog.revision = catalog.revision.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("prompt catalog revision exhausted"))?;
        let payload = serde_json::to_string(&catalog)?;
        authorize()?;
        let changed = match previous.as_deref() {
            Some(raw) => self.0.compare_exchange(tenant, NAMESPACE, &key, raw, &payload)?,
            None => self.0.insert_if_absent(tenant, NAMESPACE, &key, &payload)?,
        };
        if !changed {
            // Another first access may have won. Return its committed selection,
            // never overwrite a concurrent CRUD change or retry an old snapshot.
            let current = self.list(scope)?;
            ensure!(current.bootstrap_complete, "prompt catalog revision conflict; retry initialization");
            return Ok(current);
        }
        catalog.entries = list_entries(catalog.entries, "");
        Ok(catalog)
    }
}
