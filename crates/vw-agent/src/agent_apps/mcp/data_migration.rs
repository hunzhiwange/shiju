//! Authorized, preview-bound CC migration. Catalog persistence only.
use super::{
    McpCatalogStore, StoredCatalogRecord, catalog_create::catalog_location,
    credentials::validate_catalog_connection,
};
use crate::gateway::settings_store::GatewaySetting;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
use uuid::Uuid;
use vw_agent_apps::data_management::safety_snapshot::{
    OwnerPreimage, PresentPreimage, SafetySnapshotReceipt, SafetySnapshotScope, SnapshotCategory,
    SnapshotTarget, with_safety_snapshot,
};
use vw_agent_apps::data_management::{
    mcp::normalize,
    preview::{Decision, ImportPreview, PreviewCandidate, PreviewError, build_preview},
    source::{SourceCategory, SourceScope, SourceSnapshot},
};
use vw_api_types::mcp_management::{
    McpAppTarget, McpCatalogScope, McpCliBinding, McpFailureCode, McpNativeStatus,
};

/// Host-private prepared state. The caller must authorize source read, project
/// catalog writes and every installation before preparing and again at commit.
/// Holding this value is not write authority. No raw payload is serializable.
pub struct McpMigration {
    scope: SourceScope,
    catalog: McpCatalogScope,
    preview: ImportPreview,
    revision: String,
    observed: Vec<GatewaySetting>,
    incoming: Vec<GatewaySetting>,
}
pub struct McpMigrationCommit {
    pub snapshot: Option<SafetySnapshotReceipt>,
    pub outcome: Result<usize, McpFailureCode>,
}
impl McpMigration {
    pub fn preview(&self) -> &ImportPreview {
        &self.preview
    }
}
impl McpCatalogStore {
    pub fn preview_data_migration(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        targets: &[McpAppTarget],
    ) -> Result<McpMigration, McpFailureCode> {
        let catalog = McpCatalogScope::Project { scope: scope.project().clone() };
        let (tenant, namespace) = catalog_location(&catalog)?;
        let observed =
            self.0.list(tenant, &namespace).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let rows = source.records(scope).map_err(|_| McpFailureCode::PermissionDenied)?;
        let selected =
            rows.iter().filter(|row| row.category == SourceCategory::Mcp).collect::<Vec<_>>();
        let normalized = normalize(&selected, &catalog)?;
        let mut applications = HashMap::new();
        for target in targets {
            if target.target_host_id != scope.host_id()
                || applications.insert(target.app_id, target).is_some()
            {
                return Err(McpFailureCode::PermissionDenied);
            }
        }
        let mut incoming = Vec::new();
        for item in normalized {
            validate_catalog_connection(&item.connection)?;
            let mut bindings = Vec::new();
            for (app, enabled) in item.applications {
                // Disabled intent is preserved when an authorized installation
                // exists. Enabled intent without a resolved target must fail.
                let Some(target) = applications.get(&app) else {
                    if enabled {
                        return Err(McpFailureCode::TargetUnavailable);
                    }
                    continue;
                };
                bindings.push(McpCliBinding {
                    record: item.record.record.clone(),
                    target: (*target).clone(),
                    enabled,
                    native_status: McpNativeStatus::NotVerified,
                });
            }
            let key = item.record.record.id.0.clone();
            let stored =
                StoredCatalogRecord { record: item.record, connection: item.connection, bindings };
            incoming.push(GatewaySetting {
                config_key: key,
                payload_json: serde_json::to_string(&stored)
                    .map_err(|_| McpFailureCode::InvalidDefinition)?,
            });
        }
        incoming.sort_by(|a, b| a.config_key.cmp(&b.config_key));
        let revision = fingerprint(
            &observed.iter().map(|row| (&row.config_key, &row.payload_json)).collect::<Vec<_>>(),
        )?;
        let preview =
            build_preview(scope, source, &[SourceCategory::Mcp], revision.clone(), |_, _| {
                incoming
                    .iter()
                    .map(|row| {
                        let existing = observed.iter().find(|old| old.config_key == row.config_key);
                        // Compare the entire private definition, including executable and
                        // enablement intent, through an opaque digest. The generic portable
                        // preview otherwise excludes execution fields from its comparison.
                        Ok(PreviewCandidate {
                            incoming: comparable(&row.payload_json)?,
                            existing: existing
                                .map(|old| comparable(&old.payload_json))
                                .transpose()?,
                            references_valid: true,
                        })
                    })
                    .collect()
            })
            .map_err(preview_error)?;
        Ok(McpMigration { scope: scope.clone(), catalog, preview, revision, observed, incoming })
    }

    /// Invoke only after sensitive-write authorization and durable preimage
    /// capture under the outer migration operation. Atomic SQLite CAS protects
    /// the whole category; this method never starts MCP or writes native files.
    pub fn commit_data_migration(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        prepared: &McpMigration,
        decisions: &[(Uuid, Decision)],
        authorized_snapshot_root: &Path,
    ) -> Result<McpMigrationCommit, McpFailureCode> {
        if scope != &prepared.scope {
            return Err(McpFailureCode::PermissionDenied);
        }
        let resolved = prepared
            .preview
            .resolve(scope, source, &prepared.revision, decisions)
            .map_err(preview_error)?;
        let mut replacements = Vec::new();
        for (entry, incoming) in prepared.preview.entries().iter().zip(&prepared.incoming) {
            if resolved[&entry.id] == Decision::Skip {
                continue;
            }
            let mut stored: StoredCatalogRecord = serde_json::from_str(&incoming.payload_json)
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            if let Some(previous) =
                prepared.observed.iter().find(|row| row.config_key == incoming.config_key)
            {
                // Tombstones remain fences; migration cannot resurrect a pending
                // deletion or discard revocations.
                let old: StoredCatalogRecord = serde_json::from_str(&previous.payload_json)
                    .map_err(|_| McpFailureCode::Conflict)?;
                let next =
                    old.record.record.revision.checked_add(1).ok_or(McpFailureCode::Conflict)?;
                stored.record.record.revision = next;
                stored.record.connection_revision = next;
                for binding in &mut stored.bindings {
                    binding.record = stored.record.record.clone();
                }
            }
            replacements.push(GatewaySetting {
                config_key: incoming.config_key.clone(),
                payload_json: serde_json::to_string(&stored)
                    .map_err(|_| McpFailureCode::InvalidDefinition)?,
            });
        }
        let (tenant, namespace) = catalog_location(&prepared.catalog)?;
        if replacements.is_empty() {
            // Still enforce the observed revision for a skip-only operation.
            if !self
                .0
                .apply_mcp_migration(tenant, &namespace, &prepared.observed, &[])
                .map_err(|_| McpFailureCode::StorageUnavailable)?
            {
                return Err(McpFailureCode::StaleRevision);
            }
            return Ok(McpMigrationCommit { snapshot: None, outcome: Ok(0) });
        }
        let snapshot_scope = SafetySnapshotScope {
            project: scope.project().clone(),
            principal_id: scope.principal_id().into(),
            host_id: scope.host_id().into(),
        };
        let targets = replacements
            .iter()
            .map(|row| SnapshotTarget {
                category: SnapshotCategory::Mcp,
                object_id: row.config_key.clone(),
            })
            .collect::<Vec<_>>();
        let preimages = targets
            .iter()
            .map(|target| OwnerPreimage {
                target: target.clone(),
                state: prepared.observed.iter().find(|row| row.config_key == target.object_id).map(
                    |row| PresentPreimage {
                        revision: prepared.revision.clone(),
                        bytes: row.payload_json.as_bytes().to_vec(),
                    },
                ),
            })
            .collect();
        // Captured bytes are valid exactly when the following transactional CAS
        // succeeds. A competing change leaves the category untouched.
        let (receipt, outcome) = with_safety_snapshot(
            authorized_snapshot_root,
            &snapshot_scope,
            &targets,
            || Ok(preimages),
            |_| {
                if !self
                    .0
                    .apply_mcp_migration(tenant, &namespace, &prepared.observed, &replacements)
                    .map_err(|_| McpFailureCode::StorageUnavailable)?
                {
                    return Err(McpFailureCode::StaleRevision);
                }
                Ok(replacements.len())
            },
        )
        .map_err(|_| McpFailureCode::StorageUnavailable)?;
        Ok(McpMigrationCommit { snapshot: Some(receipt), outcome })
    }
}
fn fingerprint(value: &impl serde::Serialize) -> Result<String, McpFailureCode> {
    let bytes = serde_json::to_vec(value).map_err(|_| McpFailureCode::InvalidDefinition)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn comparable(payload: &str) -> Result<serde_json::Value, PreviewError> {
    let mut stored: StoredCatalogRecord =
        serde_json::from_str(payload).map_err(|_| PreviewError::InvalidCandidate)?;
    stored.record.record.revision = 1;
    stored.record.connection_revision = 1;
    for binding in &mut stored.bindings {
        binding.record.revision = 1;
    }
    Ok(
        serde_json::json!({"content": fingerprint(&stored).map_err(|_| PreviewError::InvalidCandidate)?}),
    )
}
fn preview_error(error: PreviewError) -> McpFailureCode {
    match error {
        PreviewError::Forbidden => McpFailureCode::PermissionDenied,
        PreviewError::Stale => McpFailureCode::StaleRevision,
        _ => McpFailureCode::InvalidDefinition,
    }
}
