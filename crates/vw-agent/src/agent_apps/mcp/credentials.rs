//! Masked catalog edits and execution-host-only credential resolution.

use std::collections::BTreeMap;

use vw_agent_apps::{
    mcp::validation::{SECRET_MASK, validate_connection},
    providers::validate_identity,
};
use vw_api_types::mcp_management::{
    McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpFailureCode, McpRecordRef,
};

use super::{McpCatalogStore, StoredCatalogEntry, catalog_create::catalog_location};

impl McpCatalogStore {
    /// Return only editor placeholders for this authorized record revision.
    pub fn credential_masks(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
    ) -> Result<BTreeMap<String, McpConfigValue>, McpFailureCode> {
        let connection = self.connection_for_revision(authorized_scope, expected)?;
        let fields = match connection {
            McpConnectionDefinition::Stdio { env, .. } => env,
            McpConnectionDefinition::StreamableHttp { headers, .. }
            | McpConnectionDefinition::Sse { headers, .. } => headers,
        };
        Ok(fields
            .into_iter()
            .filter(|(_, value)| matches!(value, McpConfigValue::Credential { .. }))
            .map(|(key, _)| (key, McpConfigValue::Literal { value: SECRET_MASK.into() }))
            .collect())
    }

    fn connection_for_revision(
        &self,
        authorized_scope: &McpCatalogScope,
        expected: &McpRecordRef,
    ) -> Result<McpConnectionDefinition, McpFailureCode> {
        let (tenant, namespace) = catalog_location(authorized_scope)?;
        let (_, entry) = self.load_entry(tenant, &namespace, &expected.id.0)?;
        let StoredCatalogEntry::Active(stored) = entry else {
            return Err(McpFailureCode::NotFound);
        };
        if stored.record.record.revision != expected.revision {
            return Err(McpFailureCode::StaleRevision);
        }
        Ok(stored.connection)
    }
}

pub(super) fn retain_credentials(
    previous: &BTreeMap<String, McpConfigValue>,
    updated: &mut BTreeMap<String, McpConfigValue>,
    same_key: fn(&str, &str) -> bool,
) {
    for (key, value) in previous {
        if !matches!(value, McpConfigValue::Credential { .. }) {
            continue;
        }
        match updated.iter_mut().find(|(new_key, _)| same_key(key, new_key)) {
            Some((_, new_value)) => {
                if matches!(new_value, McpConfigValue::Literal { value } if value == SECRET_MASK) {
                    *new_value = value.clone();
                }
            }
            None => {
                updated.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Managed definitions keep raw credential fields out of catalog storage.
pub(super) fn validate_catalog_connection(
    connection: &McpConnectionDefinition,
) -> Result<(), McpFailureCode> {
    validate_connection(connection).map_err(|_| McpFailureCode::InvalidDefinition)?;
    let invalid_value = |(key, value): (&String, &McpConfigValue)| match value {
        McpConfigValue::Literal { .. } => secret_key(key),
        McpConfigValue::Credential { reference } => {
            validate_identity(&reference.account_id).is_err()
        }
    };
    let invalid = match connection {
        McpConnectionDefinition::Stdio { env, .. } => env.iter().any(invalid_value),
        McpConnectionDefinition::StreamableHttp { url, headers }
        | McpConnectionDefinition::Sse { url, headers } => {
            let url =
                reqwest_012::Url::parse(url).map_err(|_| McpFailureCode::InvalidDefinition)?;
            !url.username().is_empty()
                || url.password().is_some()
                || url.query_pairs().any(|(key, _)| secret_key(&key))
                || headers.iter().any(invalid_value)
        }
    };
    if invalid {
        return Err(McpFailureCode::InvalidDefinition);
    }
    Ok(())
}

fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    matches!(
        key.as_str(),
        "authorization" | "proxy_authorization" | "cookie" | "set_cookie" | "apikey"
    ) || ["token", "password", "secret", "api_key"].iter().any(|suffix| {
        key == *suffix || key.strip_suffix(suffix).is_some_and(|prefix| prefix.ends_with('_'))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub use execution::McpExecutionConnection;

#[cfg(not(target_arch = "wasm32"))]
mod execution {
    use super::*;
    use crate::{
        agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
        global_database::run_sync_database,
    };
    use reqwest_012::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
    use vw_agent_apps::auth::{codex_refresh::CodexRefresh, xai_refresh::XaiRefresh};
    use vw_api_types::{
        TenantScope,
        agent_apps_auth::{AuthErrorCode, AuthService},
        mcp_management::{McpCredentialRef, McpTransport},
    };

    /// Host-private transport inputs. Keep this out of serialized settings and diagnostics.
    pub enum McpExecutionConnection {
        Stdio {
            command: String,
            args: Vec<String>,
            env: BTreeMap<String, String>,
            cwd: Option<String>,
        },
        Http {
            transport: McpTransport,
            url: String,
            headers: HeaderMap,
        },
    }

    impl McpCatalogStore {
        /// Resolve for an already authorized local execution host. The host caller must authorize
        /// use of each exact reference for this record revision and destination. The returned
        /// scope must come from trusted policy, never from the catalog definition.
        pub async fn resolve_for_execution<'a>(
            &self,
            authorized_scope: &McpCatalogScope,
            expected: &McpRecordRef,
            execution_host: &str,
            accounts: &AuthAccountStore,
            authorize_credential_use: impl Fn(
                &McpConnectionDefinition,
                &McpCredentialRef,
            ) -> Option<AuthAccountScope<'a>>
            + Send
            + Sync,
        ) -> Result<McpExecutionConnection, McpFailureCode> {
            let connection =
                run_sync_database(|| Ok(self.connection_for_revision(authorized_scope, expected)))
                    .map_err(|_| McpFailureCode::StorageUnavailable)??;
            validate_catalog_connection(&connection)?;
            let (tenant, _) = catalog_location(authorized_scope)?;
            let authorize =
                |reference: &McpCredentialRef| authorize_credential_use(&connection, reference);
            let resolve =
                |fields| resolve_fields(fields, tenant, execution_host, accounts, &authorize);
            let transport = match &connection {
                McpConnectionDefinition::Stdio { .. } => McpTransport::Stdio,
                McpConnectionDefinition::StreamableHttp { .. } => McpTransport::StreamableHttp,
                McpConnectionDefinition::Sse { .. } => McpTransport::Sse,
            };
            match &connection {
                McpConnectionDefinition::Stdio { command, args, env, cwd } => {
                    let env = resolve(env)
                        .await?
                        .into_iter()
                        .map(|(key, value, _)| {
                            if value.contains('\0') {
                                return Err(McpFailureCode::CredentialUnavailable);
                            }
                            Ok((key, value))
                        })
                        .collect::<Result<_, _>>()?;
                    Ok(McpExecutionConnection::Stdio {
                        command: command.clone(),
                        args: args.clone(),
                        env,
                        cwd: cwd.clone(),
                    })
                }
                McpConnectionDefinition::StreamableHttp { url, headers }
                | McpConnectionDefinition::Sse { url, headers } => {
                    let mut request_headers = HeaderMap::new();
                    for (key, value, sensitive) in resolve(headers).await? {
                        let name = HeaderName::from_bytes(key.as_bytes())
                            .map_err(|_| McpFailureCode::InvalidDefinition)?;
                        let text = if sensitive && name == AUTHORIZATION {
                            format!("Bearer {value}")
                        } else {
                            value
                        };
                        let mut value = HeaderValue::try_from(text)
                            .map_err(|_| McpFailureCode::CredentialUnavailable)?;
                        value.set_sensitive(sensitive);
                        request_headers.insert(name, value);
                    }
                    Ok(McpExecutionConnection::Http {
                        transport,
                        url: url.clone(),
                        headers: request_headers,
                    })
                }
            }
        }
    }

    async fn resolve_fields<'a>(
        fields: &BTreeMap<String, McpConfigValue>,
        tenant: &TenantScope,
        execution_host: &str,
        accounts: &AuthAccountStore,
        authorize: &(impl Fn(&McpCredentialRef) -> Option<AuthAccountScope<'a>> + Sync),
    ) -> Result<Vec<(String, String, bool)>, McpFailureCode> {
        let mut approved = Vec::with_capacity(fields.len());
        for (key, value) in fields {
            let scope = match value {
                McpConfigValue::Literal { .. } => None,
                McpConfigValue::Credential { reference } => Some(
                    authorize(reference)
                        .filter(|scope| {
                            scope.tenant == tenant
                                && scope.host == execution_host
                                && scope.service == reference.service
                                && scope.validate().is_ok()
                        })
                        .ok_or(McpFailureCode::PermissionDenied)?,
                ),
            };
            approved.push((key, value, scope));
        }

        let mut resolved = Vec::with_capacity(approved.len());
        for (key, value, scope) in approved {
            let (value, sensitive) = match value {
                McpConfigValue::Literal { value } => (value.clone(), false),
                McpConfigValue::Credential { reference } => {
                    let scope = scope.ok_or(McpFailureCode::PermissionDenied)?;
                    (resolve_access_token(accounts, scope, &reference.account_id).await?, true)
                }
            };
            resolved.push((key.clone(), value, sensitive));
        }
        Ok(resolved)
    }

    async fn resolve_access_token(
        accounts: &AuthAccountStore,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, McpFailureCode> {
        let result = match scope.service {
            AuthService::CodexOauth => {
                let adapter =
                    CodexRefresh::new().map_err(|_| McpFailureCode::CredentialUnavailable)?;
                accounts.codex_access_token(&adapter, scope, account_id).await
            }
            AuthService::XaiOauth => {
                let adapter =
                    XaiRefresh::new().map_err(|_| McpFailureCode::CredentialUnavailable)?;
                accounts.xai_access_token(&adapter, scope, account_id).await
            }
            _ => return Err(McpFailureCode::CredentialUnavailable),
        };
        result.map_err(|error| match error {
            AuthErrorCode::Forbidden | AuthErrorCode::NotFound => McpFailureCode::PermissionDenied,
            _ => McpFailureCode::CredentialUnavailable,
        })
    }
}
