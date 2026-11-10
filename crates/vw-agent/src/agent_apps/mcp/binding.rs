//! Apply one authorized MCP binding and retry native cleanup for deleted records.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{Mutex, MutexGuard},
};

use vw_agent_apps::mcp::{
    distribution::{NativeMcpEdit, NativeMcpWrite},
    import::NativeMcpSource,
};
use vw_agent_apps::providers::live::{commit::ProviderTransactions, write_guard::WriteError};
use vw_api_types::{
    agent_apps::{AppInstallation, AppInstallationEnvironment},
    mcp_management::{
        McpAppTarget, McpCatalogScope, McpCliBinding, McpConfigValue, McpConnectionDefinition,
        McpCredentialRef, McpDistributionItem, McpDistributionOutcome, McpFailureCode,
        McpNativeStatus, McpRecordRef, McpTransport,
    },
};

use super::{
    McpCatalogStore, McpExecutionConnection, McpRevocation, StoredCatalogEntry,
    catalog_create::catalog_location,
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::run_sync_database,
};

/// Executor input assembled after authorizing the catalog, installation and execution
/// host. The installation and native paths must come from host discovery and policy.
/// The caller must resolve the same profile associated with the target on every attempt.
pub struct McpBindingTarget<'a> {
    pub target: &'a McpAppTarget,
    pub installation: &'a AppInstallation,
    pub source: NativeMcpSource<'a>,
    pub transactions: &'a ProviderTransactions,
}

impl McpBindingTarget<'_> {
    pub(super) fn validate(&self, scope: &McpCatalogScope) -> Result<(), McpFailureCode> {
        if self.installation.app_id != self.target.app_id
            || self.installation.id != self.target.installation_id
            || self.installation.target_host_id != self.target.target_host_id
            || self.installation.environment != AppInstallationEnvironment::Macos
        {
            return Err(McpFailureCode::TargetUnavailable);
        }
        let (tenant, _) = catalog_location(scope)?;
        self.native_write(tenant).validate_target()
    }

    fn native_write<'a>(&'a self, tenant: &'a vw_api_types::TenantScope) -> NativeMcpWrite<'a> {
        NativeMcpWrite {
            tenant,
            target: self.target,
            source: self.source,
            transactions: self.transactions,
        }
    }
}

// Serialize catalog changes to a record during native publication. The adapters
// independently acquire the provider's physical application/directory write locks.
pub(super) fn lock_record(key: &str) -> MutexGuard<'static, ()> {
    static LOCKS: [Mutex<()>; 64] = [const { Mutex::new(()) }; 64];
    let mut hash = DefaultHasher::new();
    key.hash(&mut hash);
    LOCKS[hash.finish() as usize % LOCKS.len()].lock().unwrap_or_else(|e| e.into_inner())
}

impl McpCatalogStore {
    /// Persist the desired binding, then project the authorized connection on the local host.
    /// A failed projection returns the committed revision so the same target can be retried.
    pub async fn set_app_binding<'a>(
        &self,
        scope: &McpCatalogScope,
        expected: &McpRecordRef,
        target: &McpBindingTarget<'_>,
        enabled: bool,
        accounts: &AuthAccountStore,
        authorize_credential_use: impl Fn(
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
    ) -> Result<McpDistributionItem, McpFailureCode> {
        let record =
            run_sync_database(|| Ok(self.save_binding_intent(scope, expected, target, enabled)))
                .map_err(|_| McpFailureCode::StorageUnavailable)??;

        let connection = if enabled {
            self.resolve_for_execution(
                scope,
                &record,
                &target.target.target_host_id,
                accounts,
                authorize_credential_use,
            )
            .await
            .and_then(native_definition)
            .map(Some)
        } else {
            Ok(None)
        };

        let outcome = connection.and_then(|connection| {
            run_sync_database(|| {
                Ok(self.publish_binding(scope, &record, target, connection.as_ref(), false))
            })
            .map_err(|_| McpFailureCode::StorageUnavailable)?
        });
        Ok(McpDistributionItem {
            record,
            target: target.target.clone(),
            enabled,
            desired_committed: true,
            native_outcome: outcome.unwrap_or_else(|code| McpDistributionOutcome::Failed { code }),
        })
    }

    fn save_binding_intent(
        &self,
        scope: &McpCatalogScope,
        expected: &McpRecordRef,
        target: &McpBindingTarget<'_>,
        enabled: bool,
    ) -> Result<McpRecordRef, McpFailureCode> {
        target.validate(scope)?;
        let (tenant, namespace) = catalog_location(scope)?;
        let key = &expected.id.0;
        let _lock = lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Active(mut stored) = entry else {
            return Err(McpFailureCode::NotFound);
        };
        if &stored.record.record != expected {
            return Err(McpFailureCode::StaleRevision);
        }
        let index = stored.bindings.iter().position(|binding| binding.target == *target.target);
        if index.is_some_and(|index| stored.bindings[index].enabled == enabled) {
            return Ok(stored.record.record);
        }

        let record = &mut stored.record.record;
        record.revision =
            record.revision.checked_add(1).ok_or(McpFailureCode::StorageUnavailable)?;
        if let Some(index) = index {
            stored.bindings[index].enabled = enabled;
            stored.bindings[index].record = record.clone();
        } else {
            stored.bindings.push(McpCliBinding {
                record: record.clone(),
                target: target.target.clone(),
                enabled,
                native_status: McpNativeStatus::NotVerified,
            });
        }
        self.save_entry(tenant, &namespace, key, &previous, &stored)?;
        Ok(stored.record.record)
    }

    pub(super) fn publish_binding(
        &self,
        scope: &McpCatalogScope,
        record: &McpRecordRef,
        target: &McpBindingTarget<'_>,
        connection: Option<&McpConnectionDefinition>,
        restore_missing: bool,
    ) -> Result<McpDistributionOutcome, McpFailureCode> {
        let (tenant, namespace) = catalog_location(scope)?;
        let key = &record.id.0;
        let _lock = lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Active(mut stored) = entry else {
            return Err(McpFailureCode::StaleRevision);
        };
        if stored.record.record != *record {
            return Err(McpFailureCode::StaleRevision);
        }
        let binding = stored
            .bindings
            .iter_mut()
            .find(|binding| binding.target == *target.target)
            .ok_or(McpFailureCode::NotFound)?;
        let owned = binding.native_status == McpNativeStatus::Present;
        let edit = match (connection, owned) {
            (Some(connection), false) => NativeMcpEdit::Insert(connection),
            (Some(connection), true) => NativeMcpEdit::Update(connection),
            (None, true) => NativeMcpEdit::Disable,
            (None, false) => return Ok(McpDistributionOutcome::Unchanged),
        };
        let write = target.native_write(tenant);
        let preflight = || self.check_snapshot(tenant, &namespace, key, &previous);
        let status = match (edit, write.apply(key, edit, preflight)) {
            (NativeMcpEdit::Update(connection), Err(McpFailureCode::Conflict))
                if restore_missing =>
            {
                // Some adapters require an existing entry for Update. A fresh guarded Insert
                // repairs deletion while still rejecting any same-name entry.
                write.apply(key, NativeMcpEdit::Insert(connection), preflight)?
            }
            (_, result) => result?,
        };
        if binding.native_status != status || binding.record != *record {
            binding.native_status = status;
            binding.record = record.clone();
            self.save_entry(tenant, &namespace, key, &previous, &stored)?;
        }
        Ok(McpDistributionOutcome::Applied)
    }

    /// Run one saved revocation on a blocking worker after authorizing its target.
    /// Cleanup does not resolve the deleted definition's credentials.
    pub fn revoke_app_binding(
        &self,
        scope: &McpCatalogScope,
        deleted: &McpRecordRef,
        target: &McpBindingTarget<'_>,
    ) -> Result<McpRevocation, McpFailureCode> {
        target.validate(scope)?;
        let (tenant, namespace) = catalog_location(scope)?;
        let key = &deleted.id.0;
        let _lock = lock_record(key);
        let (previous, entry) = self.load_entry(tenant, &namespace, key)?;
        let StoredCatalogEntry::Deleted(mut stored) = entry else {
            return Err(McpFailureCode::StaleRevision);
        };
        if stored.deleted != *deleted {
            return Err(McpFailureCode::StaleRevision);
        }
        let revocation = stored
            .revocations
            .iter_mut()
            .find(|item| item.binding.target == *target.target)
            .ok_or(McpFailureCode::NotFound)?;
        if revocation.outcome == Some(Ok(())) {
            return Ok(revocation.clone());
        }
        let outcome = if revocation.binding.native_status == McpNativeStatus::Present {
            target
                .native_write(tenant)
                .apply(key, NativeMcpEdit::Remove, || {
                    self.check_snapshot(tenant, &namespace, key, &previous)
                })
                .map(|_| ())
        } else {
            Ok(())
        };
        revocation.outcome = Some(outcome);
        let result = revocation.clone();
        self.save_entry(tenant, &namespace, key, &previous, &stored)?;
        Ok(result)
    }

    fn check_snapshot(
        &self,
        tenant: &vw_api_types::TenantScope,
        namespace: &str,
        key: &str,
        expected: &str,
    ) -> Result<(), WriteError> {
        match self.0.get(tenant, namespace, key).map_err(|_| WriteError::Io)? {
            Some(current) if current == expected => Ok(()),
            _ => Err(WriteError::Conflict),
        }
    }
}

pub(super) fn native_definition(
    connection: McpExecutionConnection,
) -> Result<McpConnectionDefinition, McpFailureCode> {
    match connection {
        McpExecutionConnection::Stdio { command, args, env, cwd } => {
            let env = env
                .into_iter()
                .map(|(key, value)| (key, McpConfigValue::Literal { value }))
                .collect();
            Ok(McpConnectionDefinition::Stdio { command, args, env, cwd })
        }
        McpExecutionConnection::Http { transport, url, headers } => {
            let headers = headers
                .iter()
                .map(|(key, value)| {
                    let value = std::str::from_utf8(value.as_bytes())
                        .map_err(|_| McpFailureCode::CredentialUnavailable)?;
                    Ok((key.to_string(), McpConfigValue::Literal { value: value.to_owned() }))
                })
                .collect::<Result<_, _>>()?;
            match transport {
                McpTransport::StreamableHttp => {
                    Ok(McpConnectionDefinition::StreamableHttp { url, headers })
                }
                McpTransport::Sse => Ok(McpConnectionDefinition::Sse { url, headers }),
                McpTransport::Stdio => Err(McpFailureCode::InvalidDefinition),
            }
        }
    }
}
