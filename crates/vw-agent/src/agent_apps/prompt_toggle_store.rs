//! Native activation dispatch under one physical target lock and catalog CAS.
use super::*;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::{
    prompts::{activation, deactivation, mcode, pi_agents, targets},
    providers::live::{
        commit::ProviderTransactions,
        write_guard::{WriteError, lock_app},
    },
};

impl PromptsStore {
    #[allow(clippy::too_many_arguments)]
    pub fn toggle_native(
        &self,
        scope: PromptScope<'_>,
        revision: u64,
        id: &str,
        enabled: bool,
        expected: Option<&str>,
        home: &Path,
        override_dir: Option<&str>,
        environment: &BTreeMap<String, String>,
        tx: &ProviderTransactions,
        authorize: impl Fn() -> Result<(), WriteError>,
    ) -> Result<PromptCatalog> {
        authorize().map_err(native_error)?;
        let target = targets::resolve(home, scope.app, override_dir, environment)
            .map_err(anyhow::Error::msg)?;
        let mut guard = lock_app(
            scope.app,
            target.file.parent().ok_or_else(|| anyhow::anyhow!("invalid prompt target"))?,
        )
        .map_err(native_error)?;
        let name =
            target.file.file_name().ok_or_else(|| anyhow::anyhow!("invalid prompt target"))?;
        let state = tx.state(scope.app).map_err(native_error)?;
        state.require_project(scope.project).map_err(native_error)?;
        ensure!(tx.pending(scope.app).map_err(native_error)?.is_none(), "prompt recovery pending");
        let key = scope.key()?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let mut catalog = Self::decode(previous.as_deref())?;
        ensure!(catalog.revision == revision, "prompt catalog revision conflict");
        let live = guard.read(name).map_err(native_error)?;
        ensure!(live.as_deref() == expected.map(str::as_bytes), "prompt file revision conflict");
        let entry = catalog
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| anyhow::anyhow!("prompt not found"))?;
        let content = entry.content.clone();
        if scope.app == AppId::Mcode && enabled {
            mcode::validate_content(&content)
                .map_err(|_| anyhow::anyhow!("prompt content too large"))?;
        }
        let preflight = || {
            authorize()?;
            if tx.state(scope.app)? != state
                || self.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)? != previous
            {
                return Err(WriteError::Conflict);
            }
            Ok(())
        };
        let next_revision = revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("prompt catalog revision exhausted"))?;
        if scope.app == AppId::Pi {
            let entries = catalog
                .entries
                .iter()
                .map(|e| vw_api_types::agent_apps::PromptEntry {
                    id: e.id.clone(),
                    app_id: AppId::Pi,
                    name: e.name.clone(),
                    description: e.description.clone(),
                    content: e.content.clone(),
                    enabled: false,
                    created_at: e.created_at,
                    updated_at: None,
                    revision: vw_api_types::agent_apps::PromptRevision(revision.to_string()),
                })
                .collect::<Vec<_>>();
            let change = if enabled {
                pi_agents::Change::Activate(id.into())
            } else {
                pi_agents::Change::Deactivate(id.into())
            };
            catalog.revision = next_revision;
            catalog.active_id = None; // Pi selection is derived from native bytes.
            pi_agents::apply(&mut guard, tx, tenant, &entries, expected, change, preflight, |_| {
                self.commit_toggle(scope, &previous, &catalog).map_err(|_| WriteError::Conflict)
            })
            .map_err(|e| anyhow::anyhow!("Pi prompt transaction failed: {e:?}"))?;
        } else {
            if !enabled && catalog.active_id.as_deref() != Some(id) {
                preflight().map_err(native_error)?;
                return Ok(catalog);
            }
            if enabled {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis()
                    .try_into()?;
                catalog.entries = activation::prepare(
                    &catalog.entries,
                    catalog.active_id.as_deref(),
                    id,
                    None,
                    live.as_deref(),
                    now,
                )?
                .0;
                catalog.active_id = Some(id.into());
            } else {
                deactivation::refresh(
                    &mut catalog.entries,
                    catalog.active_id.as_deref(),
                    live.as_deref(),
                )?;
                catalog.active_id = None;
            }
            catalog.revision = next_revision;
            let after = if enabled { content.as_str() } else { "" };
            if !enabled && live.is_none() {
                preflight().map_err(native_error)?;
                self.commit_toggle(scope, &previous, &catalog)?;
            } else {
                match scope.app {
                    AppId::Mcode => mcode::write_and_commit(
                        &mut guard,
                        tx,
                        tenant,
                        live.as_deref(),
                        after,
                        preflight,
                        || {
                            self.commit_toggle(scope, &previous, &catalog)
                                .map_err(|_| WriteError::Conflict)
                        },
                    )
                    .map_err(|e| anyhow::anyhow!("MCode prompt transaction failed: {e:?}"))?,
                    _ => activation::publish(
                        &mut guard,
                        tx,
                        scope.project,
                        live.as_deref(),
                        after,
                        preflight,
                        || self.commit_toggle(scope, &previous, &catalog).map_err(|_| ()),
                    )?,
                }
            }
        }
        if scope.app == AppId::Pi {
            let after = if enabled {
                Some(content.as_str())
            } else if catalog
                .entries
                .iter()
                .find(|e| Some(e.content.as_str()) == expected)
                .is_some_and(|e| e.id == id)
            {
                None
            } else {
                expected
            };
            catalog.active_id = catalog
                .entries
                .iter()
                .find(|e| Some(e.content.as_str()) == after)
                .map(|e| e.id.clone());
        }
        catalog.entries = list_entries(catalog.entries, "");
        Ok(catalog)
    }
    fn commit_toggle(
        &self,
        scope: PromptScope<'_>,
        previous: &Option<String>,
        catalog: &PromptCatalog,
    ) -> Result<()> {
        let payload = serde_json::to_string(catalog)?;
        let key = scope.key()?;
        let changed = match previous.as_deref() {
            Some(raw) => self.0.compare_exchange(
                scope.project.tenant_scope(),
                NAMESPACE,
                &key,
                raw,
                &payload,
            )?,
            None => {
                self.0.insert_if_absent(scope.project.tenant_scope(), NAMESPACE, &key, &payload)?
            }
        };
        ensure!(changed, "prompt catalog revision conflict");
        Ok(())
    }
}

fn native_error(error: WriteError) -> anyhow::Error {
    anyhow::anyhow!("prompt native transaction failed: {error:?}")
}
