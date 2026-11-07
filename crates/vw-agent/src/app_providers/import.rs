//! Unified imports reuse the application decoders and existing scoped CAS saves.
use super::*;
use vw_api_types::app_providers::import::{ImportTrigger, ProviderImportResult};
impl AppProviderStore {
    #[cfg(target_os = "macos")]
    pub(crate) fn import_native(
        &self,
        tenant: &TenantScope,
        home: &Path,
        app: AppId,
        trigger: ImportTrigger,
    ) -> Result<ProviderImportResult, String> {
        use vw_agent_apps::providers::import::{ImportStrategy, NativeImport, strategy};
        let rows = self.list(tenant, app)?;
        // There are no automatic official seeds in this native catalog. Every
        // existing row is user-owned, including official-login cards.
        if strategy(app, trigger, !rows.is_empty())? == ImportStrategy::Skip {
            return Ok(ProviderImportResult { imported: 0 });
        }
        let candidates = match vw_agent_apps::providers::import::read(home, app, trigger) {
            Ok(rows) => rows,
            Err(_) if app == AppId::GrokBuild && !rows.is_empty() => {
                return Ok(ProviderImportResult { imported: 0 });
            }
            Err(_) => return Err("native provider configuration unavailable".into()),
        };
        vw_agent_apps::providers::operations::planned(candidates.len());
        // Freeze the batch before the first scoped catalog save. Once a batch
        // starts, cancellation cannot abandon its remaining catalog entries.
        if !candidates.is_empty() {
            vw_agent_apps::providers::operations::publication()?;
        }
        let mut imported = 0;
        for (id, profile) in candidates {
            // A custom Grok row blocks another default, while the official
            // seed alone does not block an explicit manual native import.
            if app == AppId::GrokBuild
                && rows.iter().any(|row| row.id != "grokbuild-official")
                && id != "grokbuild-official"
            {
                vw_agent_apps::providers::operations::advanced();
                continue;
            }
            if self
                .settings
                .get(tenant, NAMESPACE, &Self::key(app, &id)?)
                .map_err(|_| "provider storage unavailable")?
                .is_some()
            {
                vw_agent_apps::providers::operations::advanced();
                continue;
            }
            match profile {
                NativeImport::Claude(p) => {
                    self.claude_save(tenant, &id, &p, None, true)?;
                }
                NativeImport::Codex(p) => {
                    self.codex_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Gemini(p) => {
                    self.gemini_save(tenant, &id, &p, None)?;
                }
                NativeImport::Grok(p) => {
                    self.grok_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Agy(p) => {
                    self.agy_insert(tenant, &id, &p)?;
                }
                NativeImport::OpenCode(p) => {
                    self.opencode_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::OpenClaw(p) => {
                    self.openclaw_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Hermes(p) => {
                    self.hermes_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Pi(p) => {
                    self.pi_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Omp(p) => {
                    self.omp_save(tenant, &id, &p, None, None)?;
                }
                NativeImport::Mcode(p) => {
                    self.mcode_save(tenant, &id, &p, None)?;
                }
                NativeImport::Kimi(p) => {
                    self.kimi_save(tenant, &id, &p, None)?;
                }
            }
            imported += 1;
            vw_agent_apps::providers::operations::advanced();
        }
        Ok(ProviderImportResult { imported })
    }
}
