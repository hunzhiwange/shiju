//! Authorized CC migration writes to the existing catalog, never native/live files.
use super::{AppProviderStore, NAMESPACE};
use std::path::Path;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use vw_agent_apps::data_management::{
    providers::{self, ProviderPreview, ProviderRow},
    preview::Decision,
    source::{SourceScope, SourceSnapshot},
    safety_snapshot::{self, OwnerPreimage, PresentPreimage, SafetySnapshotReceipt, SafetySnapshotScope, SnapshotCategory, SnapshotTarget},
};
use vw_api_types::agent_apps::AppId;
use uuid::Uuid;

impl AppProviderStore {
    /// Caller must authorize migration for this exact project/principal/host.
    /// PostgreSQL is rejected before reads/writes until its transaction adapter exists.
    pub fn preview_cc_providers(&self, scope: &SourceScope, source: &SourceSnapshot) -> Result<ProviderPreview, String> {
        let mut connection = self.database.connect_sqlite().map_err(|_| "CC provider migration requires SQLite")?;
        let tx = connection.transaction().map_err(|_| "provider storage unavailable")?;
        let rows = read_rows(&tx, scope)?;
        providers::prepare(scope, source, &rows).map_err(|_| "invalid or unsupported CC provider migration".into())
    }

    /// One Immediate transaction covers version recheck, rollback capture and all
    /// catalog changes. Snapshot failure prevents every write; SQL failure rolls
    /// back the category. Native publication bindings are never overwritten.
    pub fn import_cc_providers(&self, scope: &SourceScope, source: &SourceSnapshot,
        preview: &ProviderPreview, decisions: &[(Uuid, Decision)], rollback_root: &Path,
    ) -> Result<(usize, Option<SafetySnapshotReceipt>), String> {
        let tenant = scope.project().tenant_scope();
        let mut connection = self.database.connect_sqlite().map_err(|_| "CC provider migration requires SQLite")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|_| "provider storage unavailable")?;
        let rows = read_rows(&tx, scope)?;
        let selected = preview.resolve(scope, source, &rows, decisions).map_err(|_| "provider preview changed or decisions invalid")?;
        if selected.is_empty() { return Ok((0, None)); }
        let mut preimages = Vec::new();
        let mut targets = Vec::new();
        for row in &selected {
            let key = providers::key(row.app, &row.id).map_err(|_| "invalid provider identity")?;
            let raw: Option<String> = tx.query_row(
                "SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",
                params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),NAMESPACE,key], |r|r.get(0))
                .optional().map_err(|_| "provider storage unavailable")?;
            let target = SnapshotTarget { category: SnapshotCategory::Provider, object_id: key };
            targets.push(target.clone());
            preimages.push(OwnerPreimage { target, state: raw.map(|raw| PresentPreimage {
                revision: raw.clone(), bytes: raw.into_bytes(),
            }) });
        }
        let receipt = safety_snapshot::create_safety_snapshot(rollback_root,
            &SafetySnapshotScope { project: scope.project().clone(), principal_id: scope.principal_id().into(), host_id: scope.host_id().into() },
            &targets, || Ok(preimages)).map_err(|_| "provider rollback snapshot unavailable")?;
        for row in &selected {
            let key = providers::key(row.app, &row.id).map_err(|_| "invalid provider identity")?;
            tx.execute("INSERT INTO gateway_settings(platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES(?1,?2,?3,?4,?5,CAST(strftime('%s','now') AS INTEGER)*1000) ON CONFLICT(platform_id,company_id,namespace,config_key) DO UPDATE SET payload_json=excluded.payload_json,updated_at_ms=excluded.updated_at_ms",
                params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),NAMESPACE,key,row.value.to_string()])
                .map_err(|_| "provider category transaction failed")?;
        }
        tx.commit().map_err(|_| "provider category transaction failed")?;
        Ok((selected.len(), Some(receipt)))
    }
}
fn read_rows(tx: &rusqlite::Transaction<'_>, scope: &SourceScope) -> Result<Vec<ProviderRow>, String> {
    let tenant = scope.project().tenant_scope();
    let mut stmt = tx.prepare("SELECT config_key,payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 ORDER BY config_key")
        .map_err(|_| "provider storage unavailable")?;
    let rows = stmt.query_map(params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),NAMESPACE], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))
        .map_err(|_| "provider storage unavailable")?;
    let mut result = Vec::new();
    for row in rows {
        let (key, raw) = row.map_err(|_| "provider storage unavailable")?;
        let (app,id) = key.split_once(':').ok_or("invalid provider identity")?;
        let app: AppId = serde_json::from_str(app).map_err(|_| "invalid provider identity")?;
        let value: serde_json::Value = serde_json::from_str(&raw).map_err(|_| "invalid provider record")?;
        // Reject reservations/bound records before bypassing their live-publication
        // protocol; migration does not acquire authority to edit those resources.
        if value.get("managedClaudeLogin").is_some() || super::codex_binding::has_codex_binding(&value)
            || value.get("claude").and_then(|v| v.get("config")).and_then(|v| v.get("account")).is_some_and(|v| !v.is_null())
            || value.get("shijuPublicationBinding").is_some() || value.get("shiju_provider_publication").is_some() {
            return Err("provider requires project publication".into());
        }
        result.push(ProviderRow { app, id: id.into(), value });
    }
    Ok(result)
}
