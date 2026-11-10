//! Preserve managed native bindings when retiring a catalog entry.

use serde::{Deserialize, Serialize};
use vw_api_types::mcp_management::{
    McpAppTarget, McpCatalogScope, McpCliBinding, McpFailureCode, McpRecordRef,
};

use super::{McpCatalogStore, StoredCatalogEntry, catalog_create::catalog_location};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct McpRevocation {
    pub binding: McpCliBinding,
    /// Absent until the first native cleanup attempt.
    pub outcome: Option<Result<(), McpFailureCode>>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct DeletedCatalogRecord {
    pub(super) deleted: McpRecordRef,
    pub(super) revocations: Vec<McpRevocation>,
}

impl McpCatalogStore {
    /// Hide the authorized catalog record and save its managed native bindings in one write.
    /// Retrying the same deletion returns the saved outcomes.
    pub fn delete(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
    ) -> Result<Vec<McpRevocation>, McpFailureCode> {
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let key = &expected.id.0;
        #[cfg(target_os = "macos")]
        let _lock = super::binding::lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let stored = match entry {
            StoredCatalogEntry::Active(stored) if &stored.record.record == expected => stored,
            StoredCatalogEntry::Deleted(stored) if &stored.deleted == expected => {
                return Ok(stored.revocations);
            }
            _ => return Err(McpFailureCode::StaleRevision),
        };
        let deleted = DeletedCatalogRecord {
            deleted: stored.record.record,
            revocations: stored
                .bindings
                .into_iter()
                .map(|binding| McpRevocation { binding, outcome: None })
                .collect(),
        };
        self.save_entry(tenant, &namespace, key, &previous, &deleted)?;
        Ok(deleted.revocations)
    }

    /// Save a native cleanup result for a target from the deleted catalog revision.
    /// Failures remain retryable. A successful cleanup is terminal.
    pub fn record_revocation_result(
        &self,
        authorized_scope: &McpCatalogScope,
        deleted: &McpRecordRef,
        target: &McpAppTarget,
        result: Result<(), McpFailureCode>,
    ) -> Result<(), McpFailureCode> {
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let key = &deleted.id.0;
        #[cfg(target_os = "macos")]
        let _lock = super::binding::lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Deleted(mut stored) = entry else {
            return Err(McpFailureCode::StaleRevision);
        };
        if &stored.deleted != deleted {
            return Err(McpFailureCode::StaleRevision);
        }
        let revocation = stored
            .revocations
            .iter_mut()
            .find(|item| &item.binding.target == target)
            .ok_or(McpFailureCode::NotFound)?;
        if revocation.outcome == Some(Ok(())) {
            return Ok(());
        }
        revocation.outcome = Some(result);
        self.save_entry(tenant, &namespace, key, &previous, &stored)
    }
}
