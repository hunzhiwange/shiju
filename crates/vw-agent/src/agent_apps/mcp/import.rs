//! Aggregate native MCP import into the authorized catalog.

use vw_agent_apps::mcp::{
    catalog::McpCatalogDraft,
    import::{self, NativeMcpCandidate, NativeMcpSource},
    validation::validate_new_name,
};
use vw_api_types::{
    TenantScope,
    mcp_management::{
        McpAppTarget, McpCatalogRecord, McpCatalogScope, McpCliBinding, McpFailureCode,
        McpNativeStatus, McpRecordId,
    },
};

use super::{
    McpCatalogStore, StoredCatalogEntry, StoredCatalogRecord, catalog_create::catalog_location,
};

#[derive(Clone)]
pub struct McpImportSource<'a> {
    pub target: McpAppTarget,
    pub native: NativeMcpSource<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct McpImportReport {
    pub target: McpAppTarget,
    /// A source-level error means its configuration could not be read.
    pub entries: Result<Vec<McpImportEntry>, McpFailureCode>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct McpImportEntry {
    /// Invalid native names are omitted from the public report.
    pub id: Option<McpRecordId>,
    pub outcome: Result<McpImportOutcome, McpFailureCode>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum McpImportOutcome {
    Added(McpCatalogRecord),
    Merged(McpCatalogRecord),
    Unchanged(McpCatalogRecord),
}

impl McpCatalogStore {
    /// Read on the authorized host and register entries in the authorized catalog scope.
    /// Call from a blocking worker after resolving and authorizing each installation.
    pub fn import_from_apps(
        &self,
        authorized_scope: &McpCatalogScope,
        sources: &[McpImportSource<'_>],
    ) -> Vec<McpImportReport> {
        sources
            .iter()
            .map(|source| McpImportReport {
                target: source.target.clone(),
                entries: catalog_location(authorized_scope).and_then(|(tenant, namespace)| {
                    import::read(tenant, &source.target, source.native).map(|candidates| {
                        candidates
                            .into_iter()
                            .map(|(name, candidate)| {
                                let id =
                                    validate_new_name(&name, &[]).ok().filter(|id| id.0 == name);
                                let outcome = match &id {
                                    Some(id) => candidate.and_then(|candidate| {
                                        self.import_candidate(
                                            authorized_scope,
                                            (tenant, &namespace),
                                            id,
                                            &source.target,
                                            candidate,
                                        )
                                    }),
                                    None => Err(McpFailureCode::InvalidDefinition),
                                };
                                McpImportEntry { id, outcome }
                            })
                            .collect()
                    })
                }),
            })
            .collect()
    }

    fn import_candidate(
        &self,
        scope: &McpCatalogScope,
        (tenant, namespace): (&TenantScope, &str),
        id: &McpRecordId,
        target: &McpAppTarget,
        candidate: NativeMcpCandidate,
    ) -> Result<McpImportOutcome, McpFailureCode> {
        let (record, connection) = McpCatalogDraft {
            id: id.0.clone(),
            name: None,
            description: None,
            homepage: None,
            documentation: None,
            tags: Vec::new(),
            connection: candidate.connection,
        }
        .into_record(scope.clone())?;
        super::credentials::validate_catalog_connection(&connection)?;

        let binding = McpCliBinding {
            record: record.record.clone(),
            target: target.clone(),
            enabled: candidate.enabled,
            native_status: McpNativeStatus::Present,
        };
        let mut new = StoredCatalogRecord { record, connection, bindings: vec![binding.clone()] };
        let _lock = super::binding::lock_record(&id.0);
        match self.insert_record(tenant, namespace, &mut new) {
            Ok(()) => return Ok(McpImportOutcome::Added(new.record)),
            Err(McpFailureCode::Conflict) => {}
            Err(error) => return Err(error),
        }

        // Re-read after a competing edit so no other target or newer definition is lost.
        for _ in 0..3 {
            let (previous, entry) = self.load_entry(tenant, namespace, &id.0)?;
            let StoredCatalogEntry::Active(mut saved) = entry else {
                return Err(McpFailureCode::Conflict);
            };
            if saved.connection != new.connection {
                return Err(McpFailureCode::Conflict);
            }
            if saved.bindings.iter().any(|binding| binding.target == *target) {
                return Ok(McpImportOutcome::Unchanged(saved.record));
            }

            let revision = &mut saved.record.record.revision;
            *revision = revision.checked_add(1).ok_or(McpFailureCode::StorageUnavailable)?;
            saved
                .bindings
                .push(McpCliBinding { record: saved.record.record.clone(), ..binding.clone() });
            match self.save_entry(tenant, namespace, &id.0, &previous, &saved) {
                Ok(()) => return Ok(McpImportOutcome::Merged(saved.record)),
                Err(McpFailureCode::StaleRevision) => {}
                Err(error) => return Err(error),
            }
        }
        Err(McpFailureCode::StaleRevision)
    }
}
