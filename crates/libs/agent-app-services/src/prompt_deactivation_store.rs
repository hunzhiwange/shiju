//! Selected prompt reconciliation on the authenticated physical host.
use super::*;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::{
    prompts::{activation, deactivation::{self, RefreshWarning}, targets},
    providers::live::{commit::{PendingTarget, ProviderTransactions},
        stage::{FileChange, WholeFile}, write_guard::{lock_app, FilePrivacy, WriteError}},
};

/// A deactivation carries the exact preview, including missing versus empty.
/// Refresh never writes the native file. Neither action accepts arbitrary paths.
pub enum OrdinaryPromptReconciliation<'a> {
    Refresh,
    Deactivate { id: &'a str, expected: Option<&'a [u8]> },
}

impl PromptsStore {
    /// The dispatcher authorizes scope/action and supplies executor-owned paths.
    /// `authorize` must recheck physical target ownership, as for activation.
    /// Returns warnings explicitly so callers can retain and display saved content.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_ordinary(
        &self, scope: PromptScope<'_>, revision: u64,
        action: OrdinaryPromptReconciliation<'_>, executor_host: &str, home: &Path,
        override_dir: Option<&str>, environment: &BTreeMap<String, String>,
        transactions: &ProviderTransactions,
        authorize: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(PromptCatalog, Option<RefreshWarning>)> {
        let key = scope.key()?;
        ensure!(!scope.host.is_empty() && scope.host == executor_host, "unauthorized prompt host");
        authorize().map_err(|_| anyhow::anyhow!("prompt authorization denied"))?;
        let name = activation::filename(scope.app)?;
        let target = targets::resolve(home, scope.app, override_dir, environment)
            .map_err(anyhow::Error::msg)?;
        let directory = target.file.parent().ok_or_else(|| anyhow::anyhow!("invalid prompt target"))?;
        let mut guard = lock_app(scope.app, directory)
            .map_err(|_| anyhow::anyhow!("prompt target lock failed"))?;
        let state = transactions.state(scope.app).map_err(|_| anyhow::anyhow!("prompt state unavailable"))?;
        state.require_project(scope.project).map_err(|_| anyhow::anyhow!("prompt target ownership conflict"))?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let mut catalog = Self::decode(previous.as_deref())?;
        ensure!(catalog.revision == revision, "prompt catalog revision conflict");
        let live = guard.read(name).map_err(|_| anyhow::anyhow!("prompt file read failed"))?;
        let deactivate = match action {
            OrdinaryPromptReconciliation::Refresh => false,
            OrdinaryPromptReconciliation::Deactivate { id, expected } => {
                ensure!(catalog.entries.iter().any(|entry| entry.id == id), "prompt not found");
                // Re-saving a disabled entry never clears an external file.
                if catalog.active_id.as_deref() != Some(id) { return Ok((catalog, None)); }
                ensure!(live.as_deref() == expected, "prompt file revision conflict");
                true
            }
        };
        let before_entries = catalog.entries.clone();
        let warning = deactivation::refresh(&mut catalog.entries, catalog.active_id.as_deref(), live.as_deref())?;
        if !deactivate && catalog.entries == before_entries { return Ok((catalog, warning)); }
        if deactivate { catalog.active_id = None; }
        catalog.revision = revision.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("prompt catalog revision exhausted"))?;
        let payload = serde_json::to_string(&catalog)?;
        let preflight = || -> Result<(), WriteError> {
            authorize()?;
            let current = self.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)?;
            if current != previous { return Err(WriteError::Conflict); }
            Ok(())
        };
        // Missing files remain missing; existing files become empty, never deleted.
        let clear = deactivate && live.is_some();
        if clear {
            let patch = WholeFile::new(live.as_deref(), Some(Vec::new()));
            transactions.commit_project_checked(scope.project, &mut guard, &state, false,
                &[FileChange { name, privacy: FilePrivacy::Private, patch: &patch }],
                PendingTarget::default(), &preflight)
                .map_err(|_| anyhow::anyhow!("prompt deactivation transaction failed"))?;
        } else {
            preflight().map_err(|_| anyhow::anyhow!("prompt reconciliation conflict"))?;
        }
        let stored = match previous.as_deref() {
            Some(raw) => self.0.compare_exchange(tenant, NAMESPACE, &key, raw, &payload),
            None => self.0.insert_if_absent(tenant, NAMESPACE, &key, &payload),
        };
        if !matches!(stored, Ok(true)) {
            if clear {
                let state = transactions.state(scope.app)
                    .map_err(|_| anyhow::anyhow!("prompt rollback state unavailable; reload required"))?;
                let patch = WholeFile::new(Some(b""), live);
                transactions.commit_project_checked(scope.project, &mut guard, &state, false,
                    &[FileChange { name, privacy: FilePrivacy::Private, patch: &patch }],
                    PendingTarget::default(), || Ok(()))
                    .map_err(|_| anyhow::anyhow!("prompt rollback conflict; reload required"))?;
            }
            bail!("prompt catalog commit failed");
        }
        catalog.entries = list_entries(catalog.entries, "");
        Ok((catalog, warning))
    }
}
