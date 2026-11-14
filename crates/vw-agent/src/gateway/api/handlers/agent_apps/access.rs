//! Administrator-owned grants. Request headers never establish host ownership.
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps_auth::AuthService, id::TenantScope};
#[cfg(target_os = "macos")]
use {
    crate::agent_apps::usage_settings::{UsageCredentialId, UsageSigningCredentialRefs},
    vw_api_types::{
        agent_apps::AppId,
        agent_apps_diagnostics::DiagnosticScope,
        apps_usage::UsageSnapshotQuery,
        id::ProjectId,
        mcp_management::{McpCatalogScope, McpConnectionDefinition, McpCredentialRef, McpRecordId},
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    principals: Vec<Principal>,
    hosts: Vec<Host>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    id: String,
    token_sha256: String,
    grants: Vec<Grant>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    #[serde(default)]
    device_sync_probe: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    device_sync_config: Vec<vw_api_types::id::ProjectId>,
    /// Explicit authority to export portable data to the configured remote.
    #[serde(default)]
    device_sync_upload: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    device_sync_download: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    data_migration_read: Vec<vw_api_types::id::ProjectId>,
    /// Explicit background authority; API backup grants alone do not schedule work.
    #[serde(default)]
    periodic_data_backups: Vec<PeriodicDataBackupPermit>,
    #[serde(default)]
    data_backups: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    data_export: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    external_sessions_read: Vec<ExternalHistoryPermit>,
    #[serde(default)]
    external_sessions_delete: Vec<ExternalHistoryPermit>,
    #[serde(default)]
    external_sessions_resume: Vec<ExternalHistoryPermit>,
    #[serde(default)]
    external_sessions_maintenance: Vec<ExternalHistoryPermit>,
    #[serde(default)]
    external_terminal_settings: bool,
    #[serde(default)]
    actions: Vec<vw_api_types::agent_apps::AppLifecycleAction>,
    #[serde(default)]
    preferences_write: bool,
    #[serde(default)]
    provider_config_read: bool,
    #[serde(default)]
    claude_config_read_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    claude_config_marker_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    claude_onboarding_projects: Vec<vw_api_types::id::ProjectId>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    aggregation_read: Vec<RoutingAppPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    aggregation_write: Vec<RoutingAppPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    aggregation_codex_restart: Vec<RoutingAppPermit>,
    #[serde(default)]
    provider_config_write: bool,
    #[serde(default)]
    mcp_catalog_write: bool,
    #[serde(default)]
    skill_inventory_read_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    skill_install_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    skill_repos_read_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    skill_repos_write_projects: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    mcp_catalog_write_projects: Vec<vw_api_types::id::ProjectId>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_refresh: Vec<UsageRefreshPermit>,
    #[serde(default)]
    usage_read: Vec<UsageReadPermit>,
    /// Project-wide canonical ledger access, independent of balance and native history grants.
    #[serde(default)]
    usage_ledger_read_projects: Vec<vw_api_types::id::ProjectId>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_settings_read: Vec<UsageReadPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_settings_write: Vec<UsageSettingsWritePermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_script_test: Vec<UsageScriptTestPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_cancel: Vec<UsageReadPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_refresh_all: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    auth_account_read: Vec<AuthService>,
    #[serde(default)]
    auth_binding_write: Vec<AuthService>,
    #[serde(default)]
    auth_default_write: Vec<AuthService>,
    #[serde(default)]
    auth_account_delete: Vec<AuthService>,
    #[serde(default)]
    auth_logout: Vec<AuthService>,
    #[serde(default)]
    auth_account_revoke: Vec<AuthService>,
    #[serde(default)]
    auth_flow_start: Vec<AuthService>,
    /// Explicit deployment assertion for direct, same-host browser clients.
    #[serde(default)]
    auth_browser_local: bool,
    #[serde(default)]
    launch: bool,
    #[serde(default)]
    hermes_dashboard: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    hermes_memory_read: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    openclaw_settings_read: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    openclaw_env_sensitive_read: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    openclaw_settings_write: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    hermes_memory_write: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    workspace_read: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    project_profiles_read: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    project_profiles_create: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    project_profiles_maintain: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    workspace_write: Vec<vw_api_types::id::ProjectId>,
    #[serde(default)]
    identities: Vec<IdentityPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    routing_read: bool,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    routing_manage: bool,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    routing_apps: Vec<RoutingAppPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    mcp_probe: Vec<McpProbePermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    mcp_acp_read: Vec<McpAcpReadPermit>,
    /// Catalog reads include one-time inert import; no native mutation authority.
    #[cfg(target_os = "macos")]
    #[serde(default)]
    prompts_read: Vec<RoutingAppPermit>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    prompts_write: Vec<RoutingAppPermit>,
    platform_id: String,
    company_id: String,
    host_id: String,
}
#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingAppPermit {
    project_id: ProjectId,
    app_id: AppId,
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpAcpReadPermit {
    project_id: ProjectId,
    session_id: vw_api_types::SessionId,
}

#[cfg(target_os = "macos")]
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct McpProbePermit {
    project_id: ProjectId,
    app_id: AppId,
    record_id: McpRecordId,
    pub connection_revision: u64,
    // SHA-256 of the canonical URL, including path and query.
    #[serde(default)]
    endpoint_sha256: String,
    #[serde(default)]
    stdio_sha256: Option<String>,
    #[serde(default)]
    pub stdio_workspace: Option<PathBuf>,
    #[serde(default)]
    tenant_catalog: bool,
    #[serde(default)]
    credentials: Vec<McpCredentialRef>,
}

#[cfg(target_os = "macos")]
impl McpProbePermit {
    fn matches(&self, scope: &DiagnosticScope, id: &McpRecordId) -> bool {
        self.project_id == *scope.project.project_id()
            && self.app_id == scope.app_id
            && self.record_id == *id
            && self.connection_revision > 0
    }

    pub fn catalog_scope(&self, scope: &DiagnosticScope) -> McpCatalogScope {
        if self.tenant_catalog {
            McpCatalogScope::Tenant { scope: scope.project.tenant_scope().clone() }
        } else {
            McpCatalogScope::Project { scope: scope.project.clone() }
        }
    }

    pub fn permits_stdio(&self, definition: &McpConnectionDefinition) -> bool {
        matches!(definition, McpConnectionDefinition::Stdio { .. })
            && self.endpoint_sha256.is_empty()
            && self.stdio_workspace.as_ref().is_some_and(|path| path.is_absolute() && path.is_dir())
            && self.stdio_sha256.as_ref().is_some_and(|digest| {
                serde_json::to_vec(definition)
                    .is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == *digest)
            })
    }

    pub fn permits_endpoint(&self, url: &str) -> bool {
        if self.stdio_sha256.is_some() || self.stdio_workspace.is_some() {
            return false;
        }
        reqwest_012::Url::parse(url).is_ok_and(|endpoint| self.matches_endpoint(&endpoint))
    }

    fn matches_endpoint(&self, endpoint: &reqwest_012::Url) -> bool {
        hex::encode(Sha256::digest(endpoint.as_str())) == self.endpoint_sha256
    }

    pub fn permits_credential(
        &self,
        definition: &McpConnectionDefinition,
        reference: &McpCredentialRef,
    ) -> bool {
        if matches!(definition, McpConnectionDefinition::Stdio { .. }) {
            return self.credentials.contains(reference) && self.permits_stdio(definition);
        }
        let url = match definition {
            McpConnectionDefinition::StreamableHttp { url, .. }
            | McpConnectionDefinition::Sse { url, .. } => url,
            _ => return false,
        };
        self.credentials.contains(reference)
            && reqwest_012::Url::parse(url).is_ok_and(|endpoint| {
                endpoint.scheme() == "https" && self.matches_endpoint(&endpoint)
            })
    }
}

#[cfg(target_os = "macos")]
pub(crate) struct McpProbeAccess {
    policy: Option<PathBuf>,
    digest: String,
}

#[cfg(target_os = "macos")]
#[derive(PartialEq, Eq)]
pub(crate) struct AuthorizedMcpProbe {
    pub principal: String,
    pub credential_root: Option<PathBuf>,
    pub permit: McpProbePermit,
}

#[cfg(target_os = "macos")]
impl McpProbeAccess {
    pub fn new(policy: Option<PathBuf>, headers: &HeaderMap) -> Result<Self, StatusCode> {
        Ok(Self { policy, digest: membership_digest(headers)? })
    }

    /// Discovery also exposes project selection metadata, requiring the existing catalog view grant.
    pub fn authorize_details(
        &self,
        scope: &DiagnosticScope,
        id: &McpRecordId,
    ) -> Result<AuthorizedMcpProbe, StatusCode> {
        let grant = self.authorize(scope, id)?;
        let view = authorize_digest(
            self.policy.as_deref(),
            &self.digest,
            scope.project.tenant_scope(),
            &scope.target_host_id,
            None,
            Capability::ProviderConfigRead,
        )?;
        if view.principal != grant.principal {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(grant)
    }

    pub fn authorize(
        &self,
        scope: &DiagnosticScope,
        id: &McpRecordId,
    ) -> Result<AuthorizedMcpProbe, StatusCode> {
        let authorized = authorize_digest(
            self.policy.as_deref(),
            &self.digest,
            scope.project.tenant_scope(),
            &scope.target_host_id,
            None,
            Capability::McpProbe(scope, id),
        )?;
        Ok(AuthorizedMcpProbe {
            principal: authorized.principal,
            credential_root: authorized.host.credential_root,
            permit: authorized.mcp_probe.ok_or(StatusCode::FORBIDDEN)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageReadPermit {
    project_id: vw_api_types::id::ProjectId,
    app_id: vw_api_types::agent_apps::AppId,
    provider_id: String,
    account_id: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageSettingsWritePermit {
    resource: UsageReadPermit,
    api_key_ref: Option<UsageCredentialId>,
    signing: Option<UsageSigningCredentialRefs>,
}

#[cfg(target_os = "macos")]
impl UsageSettingsWritePermit {
    fn matches(
        &self,
        query: &UsageSnapshotQuery,
        key: Option<UsageCredentialId>,
        signing: Option<UsageSigningCredentialRefs>,
    ) -> bool {
        self.resource.matches(query)
            && (key.is_none() || self.api_key_ref == key)
            && (signing.is_none() || self.signing == signing)
    }
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageScriptTestPermit {
    resource: UsageReadPermit,
    api_key_ref: Option<UsageCredentialId>,
    origin: String,
}

#[cfg(target_os = "macos")]
impl UsageScriptTestPermit {
    fn origin(
        &self,
        query: &UsageSnapshotQuery,
        key: Option<UsageCredentialId>,
    ) -> Option<reqwest::Url> {
        if !self.resource.matches(query)
            || key.is_some_and(|key| self.api_key_ref != Some(key))
            || vw_agent_apps::providers::validation::endpoint(&self.origin).is_err()
        {
            return None;
        }
        let url = reqwest::Url::parse(&self.origin).ok()?;
        (url.path() == "/").then_some(url)
    }
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageRefreshPermit {
    project_id: String,
    app_id: vw_api_types::agent_apps::AppId,
    provider_id: String,
    origin: String,
    account_id: Option<String>,
    kind: Option<vw_api_types::apps_usage::UsageQueryKind>,
}

#[cfg(target_os = "macos")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct UsageRefreshTarget {
    pub policy: PathBuf,
    pub project: vw_api_types::id::ProjectScope,
    pub principal: String,
    pub host: String,
    pub app: vw_api_types::agent_apps::AppId,
    pub provider: String,
    pub origin: reqwest::Url,
    pub batch: bool,
    pub codex: Option<ManagedCodexTarget>,
    pub kind: vw_api_types::apps_usage::UsageQueryKind,
}

#[cfg(target_os = "macos")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct ManagedCodexTarget {
    pub account: String,
    pub credential_root: PathBuf,
}

#[cfg(target_os = "macos")]
impl UsageRefreshTarget {
    pub(super) fn selector(&self) -> vw_api_types::apps_usage::UsageSnapshotQuery {
        use vw_api_types::apps_usage::UsageSnapshotQuery;
        UsageSnapshotQuery {
            project_id: self.project.project_id().clone(),
            target_host_id: self.host.clone(),
            app_id: self.app,
            provider_id: self.provider.clone(),
            account_id: self.codex.as_ref().map(|codex| codex.account.clone()),
            kind: Some(self.kind),
        }
    }
}

/// The administrator's explicit background grants authorize this entry point.
/// This is independent of request authentication and never uses a client token.
#[cfg(target_os = "macos")]
pub(super) fn usage_refresh_targets(path: &Path) -> Vec<UsageRefreshTarget> {
    usage_targets(path, false)
        .unwrap_or_default()
        .into_iter()
        .filter(|target| target.kind == vw_api_types::apps_usage::UsageQueryKind::CustomScript)
        .collect()
}

#[cfg(target_os = "macos")]
pub(super) fn usage_targets(
    path: &Path,
    batch: bool,
) -> Result<Vec<UsageRefreshTarget>, StatusCode> {
    use vw_api_types::apps_usage::UsageQueryKind::*;

    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for principal in policy.principals {
        for grant in principal.grants {
            let Some(host) = policy.hosts.iter().find(|host| {
                host.id == grant.host_id && host.online && matches!(host.binding, Binding::Local)
            }) else {
                continue;
            };
            let Ok(tenant) = TenantScope::new(&grant.platform_id, &grant.company_id) else {
                continue;
            };
            for permit in grant.usage_refresh {
                use vw_agent_apps::providers::{validate_identity, validation::endpoint};
                use vw_api_types::id::ProjectScope;

                let (Ok(project), Ok(origin)) = (
                    ProjectScope::new(tenant.clone(), permit.project_id.into()),
                    reqwest::Url::parse(&permit.origin),
                ) else {
                    continue;
                };
                if batch && !grant.usage_refresh_all.contains(project.project_id())
                    || validate_identity(&permit.provider_id).is_err()
                    || endpoint(&permit.origin).is_err()
                    || origin.path() != "/"
                {
                    continue;
                }
                let (codex, kind) = if let Some(account) = permit.account_id {
                    use vw_agent_apps::usage::subscriptions::codex::CODEX_USAGE_ORIGIN;
                    use vw_api_types::agent_apps::AppId;
                    let Some(credential_root) = &host.credential_root else { continue };
                    if permit.kind.is_some_and(|kind| kind != OfficialSubscription)
                        || permit.app_id != AppId::Codex
                        || origin.as_str().trim_end_matches('/') != CODEX_USAGE_ORIGIN
                        || validate_identity(&account).is_err()
                    {
                        continue;
                    }
                    (
                        Some(ManagedCodexTarget {
                            account,
                            credential_root: credential_root.clone(),
                        }),
                        OfficialSubscription,
                    )
                } else {
                    let kind = permit.kind.unwrap_or(CustomScript);
                    if kind == OfficialSubscription {
                        continue;
                    }
                    (None, kind)
                };
                let target = UsageRefreshTarget {
                    policy: path.to_owned(),
                    project,
                    principal: principal.id.clone(),
                    host: grant.host_id.clone(),
                    app: permit.app_id,
                    provider: permit.provider_id,
                    origin,
                    batch,
                    codex,
                    kind,
                };
                if seen.insert(target.clone()) {
                    targets.push(target);
                }
            }
        }
    }
    Ok(targets)
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct IdentityPermit {
    pub app_id: vw_api_types::agent_apps::AppId,
    pub kind: vw_api_types::agent_apps::LaunchIdentityKind,
    pub id: String,
}
pub(super) struct Authorization {
    external_history: Vec<ExternalHistoryPermit>,
    pub host: Host,
    pub principal: String,
    #[cfg(target_os = "macos")]
    pub(super) usage_script_origins: Vec<reqwest::Url>,
    pub identities: Vec<IdentityPermit>,
    #[cfg(target_os = "macos")]
    mcp_probe: Option<McpProbePermit>,
    native_slots: Vec<(AuthService, bool)>,
    pub(super) auth_browser_local: bool,
    pub(super) can_set_default: bool,
    pub(super) can_revoke: bool,
}
impl Authorization {
    pub(super) fn native_login_owner(&self, service: AuthService) -> Option<bool> {
        self.native_slots.iter().find(|(candidate, _)| *candidate == service).map(|(_, owns)| *owns)
    }

    pub fn permits(
        &self,
        app: vw_api_types::agent_apps::AppId,
        kind: vw_api_types::agent_apps::LaunchIdentityKind,
        id: &str,
    ) -> bool {
        self.identities.iter().any(|p| p.app_id == app && p.kind == kind && p.id == id)
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeLoginOwner {
    service: AuthService,
    platform_id: String,
    company_id: String,
    principal: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Host {
    /// Existing private directory, selected only by the administrator.
    #[serde(default)]
    pub data_backup_root: Option<PathBuf>,
    #[serde(default)]
    data_location_roots: Option<[PathBuf; 3]>,
    /// Explicit source references; never populated from desktop paths.
    #[serde(default)]
    pub data_migration_sources: std::collections::BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub data_migration_root: Option<PathBuf>,
    #[serde(default)]
    external_session_instances: Vec<ExternalHistoryInstance>,
    pub id: String,
    pub online: bool,
    pub binding: Binding,
    pub home: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub applications: PathBuf,
    #[serde(default)]
    pub credential_root: Option<PathBuf>,
    /// Administrator-selected skill SSOT on this execution host.
    #[serde(default)]
    pub skills_root: Option<PathBuf>,
    /// Explicit administrator-approved source IDs on this execution host.
    #[serde(default)]
    pub skill_install_files: std::collections::BTreeMap<String, PathBuf>,
    /// Trusted host configuration, never a request-supplied path.
    #[serde(default)]
    pub pi_directory: Option<PathBuf>,
    #[serde(default)]
    pub github_enterprise_authorities: Vec<String>,
    #[serde(default)]
    native_login_owners: Vec<NativeLoginOwner>,
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    routing_startup: Option<RoutingStartup>,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingStartup {
    enabled: bool,
    listen_address: Option<std::net::IpAddr>,
    listen_port: u16,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lifecycle {
    pub prefix: PathBuf,
    pub npm: PathBuf,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Binding {
    Local,
    Remote,
}

pub(super) fn authorize(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_action(path, headers, tenant, target, None)
}

pub(super) fn authorize_action(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
) -> Result<Host, StatusCode> {
    authorize_preferences(path, headers, tenant, target, action, false).map(|(host, _)| host)
}

pub(super) fn authorize_preferences(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
    write_host: bool,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        target,
        action,
        if write_host { Capability::PreferencesWrite } else { Capability::Read },
    )
    .map(|a| (a.host, a.principal))
}

pub(super) fn authorize_launch_session(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::Launch)
        .map(|a| (a.host, a.principal))
}
pub(super) fn authorize_identity_launch(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::Launch)
}

#[derive(Clone, Copy)]
enum Capability<'a> {
    UsageLedgerRead(&'a vw_api_types::id::ProjectId),
    DeviceSyncProbe(&'a vw_api_types::id::ProjectId),
    DeviceSyncUpload(&'a vw_api_types::id::ProjectId),
    DeviceSyncDownload(&'a vw_api_types::id::ProjectId),
    DataMigrationRead(&'a vw_api_types::id::ProjectId),
    DeviceSyncConfig(&'a vw_api_types::id::ProjectId),
    DataBackups(&'a vw_api_types::id::ProjectId),
    DataExport(&'a vw_api_types::id::ProjectId),
    ExternalHistory(&'a vw_api_types::id::ProjectId),
    ExternalHistoryMaintenance(&'a vw_api_types::id::ProjectId),
    ExternalHistoryDelete(&'a vw_api_types::id::ProjectId),
    ExternalHistoryResume(&'a vw_api_types::id::ProjectId),
    ExternalTerminalSettings,
    ProjectProfiles(&'a vw_api_types::id::ProjectId, bool),
    ProjectProfilesMaintain(&'a vw_api_types::id::ProjectId),
    OpenClawEnvRead(&'a vw_api_types::id::ProjectId),
    OpenClawSettings(&'a vw_api_types::id::ProjectId, bool),
    Workspace(&'a vw_api_types::id::ProjectId, bool),
    HermesDashboard(&'a vw_api_types::id::ProjectId),
    HermesMemory(&'a vw_api_types::id::ProjectId, bool),
    ClaudeConfig(&'a vw_api_types::id::ProjectId, vw_api_types::agent_apps_skills::claude_api::ClaudeConfigPermission),
    SkillInventory(&'a vw_api_types::id::ProjectId),
    SkillInstall(&'a vw_api_types::id::ProjectId),
    SkillRepos(&'a vw_api_types::id::ProjectId, bool),
    UsageRead(&'a vw_api_types::apps_usage::UsageSnapshotQuery),
    #[cfg(target_os = "macos")]
    UsageScript(&'a UsageSnapshotQuery, UsageScriptAction),
    #[cfg(target_os = "macos")]
    UsageCancel(&'a vw_api_types::apps_usage::UsageSnapshotQuery),
    #[cfg(target_os = "macos")]
    UsageBatch(&'a vw_api_types::id::ProjectId),
    Read,
    PreferencesWrite,
    Launch,
    ProviderConfigRead,
    #[cfg(target_os = "macos")]
    AggregationRead(&'a ProjectId, AppId),
    #[cfg(target_os = "macos")]
    PromptsRead(&'a ProjectId, AppId),
    #[cfg(target_os = "macos")]
    PromptsWrite(&'a ProjectId, AppId),
    #[cfg(target_os = "macos")]
    AggregationWrite(&'a ProjectId, AppId, bool),
    #[cfg(target_os = "macos")]
    CodexRestart(&'a ProjectId),
    McpCatalogWrite(Option<&'a vw_api_types::id::ProjectId>),
    #[cfg(target_os = "macos")]
    RoutingControl {
        scope: &'a RoutingControlScope,
        app: Option<(&'a ProjectId, AppId)>,
        manage: bool,
        provider_write: bool,
    },
    #[cfg(target_os = "macos")]
    McpProbe(&'a DiagnosticScope, &'a McpRecordId),
    #[cfg(target_os = "macos")]
    McpAcpRead(&'a ProjectId, &'a vw_api_types::SessionId),
    #[cfg(target_os = "macos")]
    DiagnosticProbe(&'a DiagnosticScope),
    #[cfg(target_os = "macos")]
    DiagnosticJob(&'a DiagnosticScope),
    ProviderConfigWrite,
    AuthAccountRead {
        service: AuthService,
        bindings: bool,
    },
    AuthBindingWrite(AuthService),
    AuthDefaultWrite(AuthService),
    AuthAccountDelete(AuthService),
    AuthAccountTerminate(AuthService, AccountTermination),
    AuthFlowStart(AuthService),
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
pub(super) enum UsageScriptAction {
    Templates,
    SettingsRead,
    SettingsWrite { key: Option<UsageCredentialId>, signing: Option<UsageSigningCredentialRefs> },
    Test(Option<UsageCredentialId>),
}

#[cfg(target_os = "macos")]
pub(super) fn authorize_usage_script(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    query: &UsageSnapshotQuery,
    action: UsageScriptAction,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        &query.target_host_id,
        None,
        Capability::UsageScript(query, action),
    )
}

pub(super) fn authorize_usage_read(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    query: &vw_api_types::apps_usage::UsageSnapshotQuery,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        &query.target_host_id,
        None,
        Capability::UsageRead(query),
    )
}

#[cfg(target_os = "macos")]
pub(super) fn authorize_usage_refresh(
    path: &Path,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    host: &str,
    batch: bool,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        Some(path),
        headers,
        project.tenant_scope(),
        host,
        None,
        if batch { Capability::UsageBatch(project.project_id()) } else { Capability::Read },
    )
}

#[cfg(target_os = "macos")]
pub(super) fn authorize_usage_cancel(
    path: &Path,
    headers: &HeaderMap,
    tenant: &TenantScope,
    query: &vw_api_types::apps_usage::UsageSnapshotQuery,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        Some(path),
        headers,
        tenant,
        &query.target_host_id,
        None,
        Capability::UsageCancel(query),
    )
}

impl UsageReadPermit {
    fn matches(&self, query: &vw_api_types::apps_usage::UsageSnapshotQuery) -> bool {
        self.project_id == query.project_id
            && self.app_id == query.app_id
            && self.provider_id == query.provider_id
            && self.account_id == query.account_id
    }
}

pub(super) fn authorize_auth_account_read(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
    bindings: bool,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        target,
        None,
        Capability::AuthAccountRead { service, bindings },
    )
}

pub(super) fn authorize_auth_binding(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::AuthBindingWrite(service))
}

pub(super) fn authorize_auth_default(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::AuthDefaultWrite(service))
}

pub(super) fn authorize_auth_account_delete(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::AuthAccountDelete(service))
}

#[derive(Clone, Copy)]
pub(in crate::gateway::api::handlers) enum AccountTermination {
    Logout,
    Revoke,
}

pub(super) fn authorize_auth_account_termination(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
    action: AccountTermination,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        target,
        None,
        Capability::AuthAccountTerminate(service, action),
    )
}

pub(super) fn authorize_auth_flow_start(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    service: AuthService,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::AuthFlowStart(service))
}

pub(super) fn tenant_policy_path(tenant: &TenantScope) -> PathBuf {
    vw_shared::local_apps::policy_path(
        &crate::global::paths().home,
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
    )
}

fn authorize_scoped(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
    capability: Capability<'_>,
) -> Result<Authorization, StatusCode> {
    authorize_digest(path, &membership_digest(headers)?, tenant, target, action, capability)
}

fn membership_digest(headers: &HeaderMap) -> Result<String, StatusCode> {
    let mut values = headers.get_all("x-agent-apps-token").iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(hex::encode(Sha256::digest(token)))
}

fn authorize_digest(
    path: Option<&Path>,
    digest: &str,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
    capability: Capability<'_>,
) -> Result<Authorization, StatusCode> {
    let local_path;
    let path = match path {
        Some(path) => path,
        None => {
            local_path = tenant_policy_path(tenant);
            &local_path
        }
    };
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let principal = policy
        .principals
        .iter()
        .find(|principal| {
            crate::security::pairing::constant_time_eq(&principal.token_sha256, digest)
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // Do not reveal whether an unauthorized host exists.
    let mut scoped_grants = principal.grants.iter().filter(|g| {
        g.platform_id == tenant.platform_id().as_ref()
            && g.company_id == tenant.company_id().as_ref()
            && g.host_id == target
    });
    let grants: Vec<_> = scoped_grants
        .clone()
        .filter(|g| {
            action.is_none_or(|a| g.actions.contains(&a))
                && match capability {
                    Capability::DeviceSyncProbe(project) => g.device_sync_probe.contains(project),
                    Capability::DataMigrationRead(project) => g.data_migration_read.contains(project),
                    Capability::DeviceSyncUpload(project) => g.device_sync_upload.contains(project),
                    Capability::DeviceSyncDownload(project) => g.device_sync_download.contains(project),
                    Capability::DeviceSyncConfig(project) => g.device_sync_config.contains(project),
                    Capability::DataBackups(project) => g.data_backups.contains(project),
                    Capability::DataExport(project) => g.data_export.contains(project),
                    Capability::UsageRead(query) => {
                        g.usage_read.iter().any(|permit| permit.matches(query))
                    }
                    #[cfg(target_os = "macos")]
                    Capability::UsageScript(query, UsageScriptAction::Templates) => {
                        g.usage_read.iter().any(|permit| permit.matches(query))
                    }
                    #[cfg(target_os = "macos")]
                    Capability::UsageScript(query, UsageScriptAction::SettingsRead) => {
                        g.usage_settings_read.iter().any(|permit| permit.matches(query))
                    }
                    #[cfg(target_os = "macos")]
                    Capability::UsageScript(
                        query,
                        UsageScriptAction::SettingsWrite { key, signing },
                    ) => g
                        .usage_settings_write
                        .iter()
                        .any(|permit| permit.matches(query, key, signing)),
                    #[cfg(target_os = "macos")]
                    Capability::UsageScript(query, UsageScriptAction::Test(key)) => {
                        g.usage_script_test.iter().any(|permit| permit.origin(query, key).is_some())
                    }
                    #[cfg(target_os = "macos")]
                    Capability::UsageCancel(query) => {
                        g.usage_cancel.iter().any(|permit| permit.matches(query))
                    }
                    #[cfg(target_os = "macos")]
                    Capability::UsageBatch(project) => g.usage_refresh_all.contains(project),
                    Capability::OpenClawEnvRead(project) => g.openclaw_env_sensitive_read.contains(project),
                    Capability::OpenClawSettings(project, write) => if write { g.openclaw_settings_write.contains(project) } else { g.openclaw_settings_read.contains(project) },
                    Capability::ProjectProfilesMaintain(project) => g.project_profiles_maintain.contains(project),
                    Capability::ProjectProfiles(project, create) => if create { g.project_profiles_create.contains(project) } else { g.project_profiles_read.contains(project) },
                    Capability::UsageLedgerRead(project) => g.usage_ledger_read_projects.contains(project),
                    Capability::ExternalHistory(project) => g.external_sessions_read.iter().any(|p| &p.project_id == project),
                    Capability::ExternalHistoryMaintenance(project) => g.external_sessions_maintenance.iter().any(|p| &p.project_id == project),
                    Capability::ExternalHistoryDelete(project) => g.external_sessions_delete.iter().any(|p| &p.project_id == project),
                    Capability::ExternalHistoryResume(project) => g.external_sessions_resume.iter().any(|p| &p.project_id == project),
                    Capability::ExternalTerminalSettings => g.external_terminal_settings,
                    Capability::Read => true,
                    Capability::PreferencesWrite => g.preferences_write,
                    Capability::Launch => g.launch,
                    Capability::Workspace(project, write) => if write { g.workspace_write.contains(project) } else { g.workspace_read.contains(project) },
                    Capability::HermesMemory(project, write) => if write { g.hermes_memory_write.contains(project) } else { g.hermes_memory_read.contains(project) },
                    Capability::HermesDashboard(project) => g.launch && g.hermes_dashboard.contains(project),
                    Capability::ProviderConfigRead => g.provider_config_read,
                    #[cfg(target_os = "macos")]
                    Capability::CodexRestart(project) => g
                        .aggregation_codex_restart
                        .iter()
                        .any(|p| &p.project_id == project && p.app_id == AppId::Codex),
                    #[cfg(target_os = "macos")]
                    Capability::PromptsWrite(project, app) => g.prompts_write.iter()
                        .any(|permit| &permit.project_id == project && permit.app_id == app),
                    #[cfg(target_os = "macos")]
                    Capability::PromptsRead(project, app) => g.prompts_read.iter()
                        .any(|permit| &permit.project_id == project && permit.app_id == app),
                    #[cfg(target_os = "macos")]
                    Capability::AggregationRead(project, app) => {
                        g.provider_config_read
                            && g.aggregation_read
                                .iter()
                                .any(|permit| &permit.project_id == project && permit.app_id == app)
                    }
                    #[cfg(target_os = "macos")]
                    Capability::AggregationWrite(project, app, attached) => {
                        g.provider_config_write
                            && g.aggregation_write
                                .iter()
                                .any(|p| &p.project_id == project && p.app_id == app)
                            && (!attached
                                || g.routing_manage
                                    && g.routing_apps
                                        .iter()
                                        .any(|p| &p.project_id == project && p.app_id == app))
                    }
                    Capability::ClaudeConfig(project, permission) => {
                        use vw_api_types::agent_apps_skills::claude_api::ClaudeConfigPermission;
                        match permission {
                            ClaudeConfigPermission::Read => g.claude_config_read_projects.contains(project),
                            ClaudeConfigPermission::Marker => g.claude_config_marker_projects.contains(project),
                            ClaudeConfigPermission::Onboarding => g.claude_onboarding_projects.contains(project),
                        }
                    }
                    Capability::SkillInventory(project) => g.skill_inventory_read_projects.contains(project),
                    Capability::SkillInstall(project) => g.skill_install_projects.contains(project),
                    Capability::SkillRepos(project, write) => {
                        if write { g.skill_repos_write_projects.contains(project) }
                        else { g.skill_repos_read_projects.contains(project) }
                    }
                    Capability::McpCatalogWrite(project) => match project {
                        Some(project) => g.mcp_catalog_write_projects.contains(project),
                        None => g.mcp_catalog_write,
                    },
                    #[cfg(target_os = "macos")]
                    Capability::RoutingControl { scope, app, manage, provider_write } => {
                        scope.tenant == *tenant
                            && scope.host == target
                            && (!provider_write || g.provider_config_write)
                            && (if manage { g.routing_manage } else { g.routing_read })
                            && app.is_none_or(|(project_id, app_id)| {
                                g.routing_apps.iter().any(|permit| {
                                    &permit.project_id == project_id && permit.app_id == app_id
                                })
                            })
                    }
                    #[cfg(target_os = "macos")]
                    Capability::McpAcpRead(project, session) => {
                        g.mcp_acp_read.iter().any(|permit| {
                            &permit.project_id == project && &permit.session_id == session
                        })
                    }
                    #[cfg(target_os = "macos")]
                    Capability::McpProbe(scope, id) => {
                        g.mcp_probe.iter().any(|permit| permit.matches(scope, id))
                    }
                    #[cfg(target_os = "macos")]
                    Capability::DiagnosticJob(scope) => {
                        // Reuse existing diagnostic grants, without requiring a new permission.
                        // The registry separately requires the exact submitting principal and scope.
                        (g.provider_config_read
                            && g.routing_manage
                            && g.routing_apps.iter().any(|permit| {
                                permit.project_id == *scope.project.project_id()
                                    && permit.app_id == scope.app_id
                            }))
                            || g.mcp_probe.iter().any(|permit| {
                                permit.project_id == *scope.project.project_id()
                                    && permit.app_id == scope.app_id
                                    && permit.connection_revision > 0
                            })
                    }
                    #[cfg(target_os = "macos")]
                    Capability::DiagnosticProbe(scope) => {
                        g.provider_config_read
                            && g.routing_manage
                            && g.routing_apps.iter().any(|permit| {
                                permit.project_id == *scope.project.project_id()
                                    && permit.app_id == scope.app_id
                            })
                    }
                    Capability::ProviderConfigWrite => g.provider_config_write,
                    Capability::AuthBindingWrite(service) => {
                        g.provider_config_write && g.auth_binding_write.contains(&service)
                    }
                    Capability::AuthDefaultWrite(service) => {
                        g.auth_default_write.contains(&service)
                    }
                    Capability::AuthAccountDelete(service) => {
                        g.auth_account_delete.contains(&service)
                    }
                    Capability::AuthAccountTerminate(service, AccountTermination::Logout) => {
                        g.auth_logout.contains(&service)
                    }
                    Capability::AuthAccountTerminate(service, AccountTermination::Revoke) => {
                        g.auth_account_revoke.contains(&service)
                    }
                    Capability::AuthFlowStart(service) => g.auth_flow_start.contains(&service),
                    Capability::AuthAccountRead { service, bindings } => {
                        g.auth_account_read.contains(&service)
                            && (!bindings || g.provider_config_read)
                    }
                }
        })
        .collect();
    if grants.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }
    let (can_set_default, can_revoke) = match capability {
        Capability::AuthAccountRead { service, .. } => (
            scoped_grants.clone().any(|grant| grant.auth_default_write.contains(&service)),
            scoped_grants.any(|grant| grant.auth_account_revoke.contains(&service)),
        ),
        _ => (false, false),
    };
    let auth_browser_local = grants.iter().any(|grant| grant.auth_browser_local);
    #[cfg(target_os = "macos")]
    let usage_script_origins = match capability {
        Capability::UsageScript(query, UsageScriptAction::Test(key)) => grants
            .iter()
            .flat_map(|grant| &grant.usage_script_test)
            .filter_map(|permit| permit.origin(query, key))
            .collect(),
        _ => Vec::new(),
    };
    #[cfg(target_os = "macos")]
    let mcp_probe = if let Capability::McpProbe(scope, id) = capability {
        let mut permits = grants
            .iter()
            .flat_map(|grant| &grant.mcp_probe)
            .filter(|permit| permit.matches(scope, id));
        let permit =
            permits.next().filter(|_| permits.next().is_none()).ok_or(StatusCode::FORBIDDEN)?;
        Some(permit.clone())
    } else {
        None
    };
    let external_history = match capability {
        Capability::ExternalHistoryResume(project) => grants.iter().flat_map(|g| &g.external_sessions_resume).filter(|p| &p.project_id == project).cloned().collect(),
        Capability::ExternalHistoryMaintenance(project) => grants.iter().flat_map(|g| &g.external_sessions_maintenance).filter(|p| &p.project_id == project).cloned().collect(),
        Capability::ExternalHistoryDelete(project) => grants.iter().flat_map(|g| &g.external_sessions_delete).filter(|p| &p.project_id == project).cloned().collect(),
        Capability::ExternalHistory(project) => grants.iter().flat_map(|g| &g.external_sessions_read).filter(|p| &p.project_id == project).cloned().collect(),
        _ => Vec::new(),
    };
    let identities = grants.into_iter().flat_map(|g| g.identities.clone()).collect();
    let native_slots = policy
        .hosts
        .iter()
        .flat_map(|host| {
            host.native_login_owners.iter().map(|owner| {
                let owns = host.id == target
                    && owner.principal == principal.id
                    && owner.platform_id == tenant.platform_id().as_ref()
                    && owner.company_id == tenant.company_id().as_ref();
                (owner.service, owns)
            })
        })
        .collect();
    let host = policy.hosts.into_iter().find(|h| h.id == target).ok_or(StatusCode::NOT_FOUND)?;
    if !host.online {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if matches!(host.binding, Binding::Remote) {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    Ok(Authorization {
        external_history,
        host,
        principal: principal.id.clone(),
        #[cfg(target_os = "macos")]
        usage_script_origins,
        identities,
        #[cfg(target_os = "macos")]
        mcp_probe,
        native_slots,
        auth_browser_local,
        can_set_default,
        can_revoke,
    })
}

fn load(path: &Path) -> Result<Policy, ()> {
    if !path.is_absolute() {
        return Err(());
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(())
    }
    #[cfg(unix)]
    {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path.parent().ok_or(())?)
            .map_err(|_| ())?;
        let parent = directory.metadata().map_err(|_| ())?;
        // The configured directory must not allow another OS principal to replace grants.
        let uid = unsafe { libc::geteuid() };
        if !parent.is_dir()
            || parent.mode() & 0o022 != 0
            || (parent.uid() != uid && parent.uid() != 0)
        {
            return Err(());
        }
        use std::os::unix::ffi::OsStrExt;
        let name =
            std::ffi::CString::new(path.file_name().ok_or(())?.as_bytes()).map_err(|_| ())?;
        // Keep the validated directory open so path replacement cannot change its identity.
        // SAFETY: directory is live and name is a NUL-terminated single path component.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(());
        }
        // SAFETY: openat returned an owned descriptor, consumed exactly once here.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let metadata = file.metadata().map_err(|_| ())?;
        if !metadata.is_file()
            || metadata.mode() & 0o077 != 0
            || (metadata.uid() != uid && metadata.uid() != 0)
            || metadata.len() > 65536
        {
            return Err(());
        }
        let mut bytes = Vec::new();
        file.take(65537).read_to_end(&mut bytes).map_err(|_| ())?;
        if bytes.len() > 65536 {
            return Err(());
        }
        let policy: Policy = serde_json::from_slice(&bytes).map_err(|_| ())?;
        let mut ids = HashSet::new();
        let mut services = Vec::new();
        for host in &policy.hosts {
            if host.id.trim().is_empty()
                || !ids.insert(&host.id)
                || !host.home.is_absolute()
                || !host.applications.is_absolute()
                || host.pi_directory.as_ref().is_some_and(|directory| {
                    !directory.is_absolute() || !directory.starts_with(&host.home)
                })
                || host.credential_root.as_ref().is_some_and(|root| !root.is_absolute())
                || host.native_login_owners.iter().any(|owner| {
                    let unique = !services.contains(&owner.service);
                    services.push(owner.service);
                    !unique
                        || !matches!(
                            owner.service,
                            AuthService::ClaudeOfficial
                                | AuthService::AgyOauth
                                | AuthService::CodexOauth
                                | AuthService::KimiOauth
                                | AuthService::DshAccount
                        )
                        || !policy.principals.iter().any(|principal| {
                            principal.id == owner.principal
                                && principal.grants.iter().any(|grant| {
                                    grant.host_id == host.id
                                        && grant.platform_id == owner.platform_id
                                        && grant.company_id == owner.company_id
                                })
                        })
                })
                || host.search_paths.len() > 64
                || host.search_paths.iter().any(|p| !p.is_absolute())
            {
                return Err(());
            }
        }
        let mut principals = HashSet::new();
        let mut digests = HashSet::new();
        for principal in &policy.principals {
            if principal.id.trim().is_empty()
                || !principals.insert(&principal.id)
                || !digests.insert(&principal.token_sha256)
                || principal.token_sha256.len() != 64
                || !principal
                    .token_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || principal.grants.iter().any(|g| {
                    g.platform_id.trim().is_empty()
                        || g.company_id.trim().is_empty()
                        || !ids.contains(&g.host_id)
                        || g.identities
                            .iter()
                            .any(|p| vw_agent_apps::providers::validate_identity(&p.id).is_err())
                })
            {
                return Err(());
            }
        }
        Ok(policy)
    }
}

pub(in super::super) fn authorize_mcp_catalog_write(
    path: Option<&Path>,
    headers: &HeaderMap,
    scope: &vw_api_types::mcp_management::McpCatalogScope,
    target: &str,
) -> Result<String, StatusCode> {
    use vw_api_types::mcp_management::McpCatalogScope;

    let (tenant, project) = match scope {
        McpCatalogScope::Tenant { scope } => (scope, None),
        McpCatalogScope::Project { scope } => (scope.tenant_scope(), Some(scope.project_id())),
    };
    authorize_scoped(path, headers, tenant, target, None, Capability::McpCatalogWrite(project))
        .map(|authorization| authorization.principal)
}

/// Configuration metadata reads require an independent administrator grant.
pub(super) fn authorize_provider_config(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigRead)
        .map(|a| a.host)
}

#[cfg(target_os = "macos")]
pub(super) fn authorize_provider_discovery(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigRead)
        .map(|a| (a.host, a.principal))
}

#[cfg(target_os = "macos")]
pub(crate) struct RoutingControlScope {
    pub tenant: TenantScope,
    pub host: String,
    pub bind: std::net::SocketAddr,
}

#[cfg(target_os = "macos")]
pub(crate) struct RoutingSetup {
    pub startup_address: Option<std::net::SocketAddr>,
    pub control: Option<RoutingControlScope>,
}

/// One explicit administrator policy governs the gateway's single listener.
#[cfg(target_os = "macos")]
pub(crate) fn routing_setup(path: &Path) -> Result<RoutingSetup, ()> {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let bind = |config: Option<&RoutingStartup>| {
        let ip = config
            .and_then(|config| config.listen_address)
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
            .to_canonical();
        let port = config.map_or(15721, |config| config.listen_port);
        if !ip.is_loopback() || (1..1024).contains(&port) {
            return Err(());
        }
        Ok(SocketAddr::new(ip, port))
    };
    let policy = load(path)?;
    let mut enabled = policy.hosts.iter().filter(|host| {
        host.online
            && matches!(host.binding, Binding::Local)
            && host.routing_startup.as_ref().is_some_and(|startup| startup.enabled)
    });
    let startup = enabled.next();
    if enabled.next().is_some() {
        return Err(());
    }
    let startup_address = startup.map(|host| bind(host.routing_startup.as_ref())).transpose()?;

    let mut grants = policy
        .principals
        .iter()
        .flat_map(|principal| &principal.grants)
        .filter(|grant| grant.routing_read || grant.routing_manage);
    let control = if let Some(owner) = grants.next() {
        if startup.is_some_and(|host| host.id != owner.host_id)
            || grants.any(|grant| {
                grant.platform_id != owner.platform_id
                    || grant.company_id != owner.company_id
                    || grant.host_id != owner.host_id
            })
        {
            return Err(());
        }
        let tenant = TenantScope::new(owner.platform_id.clone(), owner.company_id.clone())
            .map_err(|_| ())?;
        let host = policy.hosts.iter().find(|host| host.id == owner.host_id).ok_or(())?;
        vw_agent_apps::providers::validate_identity(&host.id).map_err(|_| ())?;
        Some(RoutingControlScope {
            tenant,
            host: host.id.clone(),
            bind: bind(host.routing_startup.as_ref())?,
        })
    } else {
        None
    };
    Ok(RoutingSetup { startup_address, control })
}

#[cfg(target_os = "macos")]
pub(crate) fn authorize_routing_control(
    path: &Path,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    scope: &RoutingControlScope,
    app: Option<(&ProjectId, AppId)>,
    manage: bool,
) -> Result<(Option<PathBuf>, String), StatusCode> {
    authorize_scoped(
        Some(path),
        headers,
        tenant,
        target,
        None,
        Capability::RoutingControl { scope, app, manage, provider_write: false },
    )
    .map(|authorized| (authorized.host.credential_root, authorized.principal))
}

/// Authorize the provider write and the exact project/application route in one policy snapshot.
#[cfg(target_os = "macos")]
pub(crate) fn authorize_routing_mode(
    path: &Path,
    headers: &HeaderMap,
    scope: &RoutingControlScope,
    project: &vw_api_types::id::ProjectScope,
    host: &str,
    app: AppId,
) -> Result<(PathBuf, String), StatusCode> {
    let authorization = authorize_scoped(
        Some(path),
        headers,
        project.tenant_scope(),
        host,
        None,
        Capability::RoutingControl {
            scope,
            app: Some((project.project_id(), app)),
            manage: true,
            provider_write: true,
        },
    )?;
    let root = authorization.host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((root, authorization.principal))
}

/// Startup consumes the same administrator-owned host identities and private
/// roots as account execution. No request path, token or ambient HOME fallback.
#[cfg(target_os = "macos")]
pub(super) fn recover_provider_policy(path: &Path) -> Result<(), ()> {
    use vw_agent_apps::providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        recovery::RecoveryOutcome,
        write_guard::WriteError,
    };
    for host in load(path)?.hosts {
        if !host.online || !matches!(host.binding, Binding::Local) {
            continue;
        }
        let Some(root) = host.credential_root else {
            continue;
        };
        // Unused hosts have no transactions. Do not provision a root at startup.
        if !root.join("provider-live").try_exists().map_err(|_| ())? {
            continue;
        }
        let transactions =
            ProviderTransactions::new(&root, ExecutionHostId::new(&host.id).map_err(|_| ())?)
                .map_err(|_| ())?;
        let login = if host.home.join(".claude").exists() {
            Some(
                vw_agent_apps::providers::claude_login::LoginHost::open(&host.home)
                    .map_err(|_| ())?,
            )
        } else {
            None
        };
        if let Some(login) = &login
            && !login.journal_matches_host(&transactions).map_err(|_| ())?
        {
            tracing::warn!(
                "provider recovery deferred; login intent belongs to another execution host"
            );
            continue;
        }
        let keychain = vw_agent_apps::auth::claude_keychain::MacOsKeychain;
        // Validate the login journal before provider recovery can publish the pointer.
        let settle_login = match login.as_ref().map(|login| login.recover(&transactions, &keychain))
        {
            Some(Err(WriteError::Recovered)) => login.as_ref(),
            Some(Err(_)) => return Err(()),
            Some(Ok(())) | None => None,
        };
        for (app, outcome) in transactions.recover_on_startup() {
            match outcome {
                Ok(RecoveryOutcome::Discarded) => {
                    tracing::info!(?app, "discarded unpublished provider transaction")
                }
                Ok(RecoveryOutcome::RolledForward) => {
                    tracing::info!(?app, "recovered provider transaction")
                }
                Ok(RecoveryOutcome::RolledForwardExcept { names }) => tracing::warn!(
                    ?app,
                    skipped = names.len(),
                    "partially recovered provider transaction; external files preserved"
                ),
                Err(_) => tracing::warn!(?app, "provider recovery failed; pending intent retained"),
            }
        }
        if let Some(login) = settle_login {
            login.recover(&transactions, &keychain).map_err(|_| ())?;
        }
    }
    Ok(())
}

pub(super) fn authorize_provider_write(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_provider_editor(path, headers, tenant, target, true).map(|a| a.host)
}

pub(super) fn authorize_provider_editor(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    write: bool,
) -> Result<Authorization, StatusCode> {
    let capability =
        if write { Capability::ProviderConfigWrite } else { Capability::ProviderConfigRead };
    authorize_scoped(path, headers, tenant, target, None, capability)
}

/// Configuration writes do not grant access to every registered official identity.
pub(super) fn authorize_provider_login(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    account: &str,
) -> Result<(), StatusCode> {
    let authorization =
        authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigWrite)?;
    if !authorization.permits(
        vw_api_types::agent_apps::AppId::Claude,
        vw_api_types::agent_apps::LaunchIdentityKind::OfficialAccount,
        account,
    ) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

/// Import only empty exclusive catalogs for administrator-authorized startup hosts.
#[cfg(target_os = "macos")]
pub(super) fn import_provider_policy(
    path: &Path,
    database: &crate::global_database::GlobalDatabase,
) -> Result<(), ()> {
    use vw_api_types::{agent_apps::AppId, app_providers::import::ImportTrigger};
    let policy = load(path)?;
    for principal in &policy.principals {
        for grant in &principal.grants {
            if !grant.provider_config_read || !grant.provider_config_write {
                continue;
            }
            let Some(host) = policy
                .hosts
                .iter()
                .find(|h| h.id == grant.host_id && h.online && matches!(h.binding, Binding::Local))
            else {
                continue;
            };
            let tenant = TenantScope::new(&grant.platform_id, &grant.company_id).map_err(|_| ())?;
            let store =
                crate::app_providers::AppProviderStore::new(database.clone()).map_err(|_| ())?;
            for app in [AppId::Claude, AppId::Codex, AppId::Gemini, AppId::GrokBuild, AppId::Agy] {
                if store.import_native(&tenant, &host.home, app, ImportTrigger::Startup).is_err() {
                    // Missing, unsupported and proxy-owned native files are all
                    // deferred. Never log their bytes or error payloads.
                    tracing::debug!(?app, "native provider startup import deferred");
                }
            }
        }
    }
    Ok(())
}

/// Read authorization never grants native slot ownership or permission to publish.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_aggregation_read(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    target: &str,
    app: AppId,
) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode> {
    authorize_aggregation(
        path,
        headers,
        project,
        target,
        Capability::AggregationRead(project.project_id(), app),
    )
    .map(|authorization| {
        (authorization.host.home, authorization.host.credential_root, authorization.principal)
    })
}
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_aggregation_write(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    target: &str,
    app: AppId,
    attached: bool,
) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode> {
    authorize_aggregation(
        path,
        headers,
        project,
        target,
        Capability::AggregationWrite(project.project_id(), app, attached),
    )
    .map(|authorization| {
        (authorization.host.home, authorization.host.credential_root, authorization.principal)
    })
}
#[cfg(target_os = "macos")]
fn authorize_aggregation(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    target: &str,
    capability: Capability<'_>,
) -> Result<Authorization, StatusCode> {
    // These routes accept a single Bearer credential under the existing policy.
    let mut values = headers.get_all(axum::http::header::AUTHORIZATION).iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    authorize_digest(
        path,
        &hex::encode(Sha256::digest(token)),
        project.tenant_scope(),
        target,
        None,
        capability,
    )
    .map_err(|status| {
        if status == StatusCode::NOT_IMPLEMENTED {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            status
        }
    })
}

/// Resolve current diagnostic membership before looking up a task. An ID grants no access.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_diagnostic_job(
    path: Option<&Path>,
    headers: &HeaderMap,
    scope: &DiagnosticScope,
) -> Result<String, StatusCode> {
    authorize_scoped(
        path,
        headers,
        scope.project.tenant_scope(),
        &scope.target_host_id,
        None,
        Capability::DiagnosticJob(scope),
    )
    .map(|authorization| authorization.principal)
}

/// Diagnostics use the existing application management and saved-provider read grants.
/// Model selection is additionally checked against the saved deployment by the executor.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_diagnostic_probe(
    path: Option<&Path>,
    headers: &HeaderMap,
    scope: &DiagnosticScope,
    request: &vw_api_types::agent_apps_diagnostics::DiagnosticStartRequest,
) -> Result<(Option<PathBuf>, String), StatusCode> {
    if let vw_api_types::agent_apps_diagnostics::DiagnosticStartRequest::Mcp { server_ids } =
        request
    {
        if server_ids.is_empty()
            || server_ids.len() > crate::gateway::agent_apps_diagnostic_jobs::MAX_PROBES_PER_JOB
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        // Authorize the entire batch before any configuration or credential lookup.
        let mut authorized = None;
        let mut unique = HashSet::new();
        for id in server_ids {
            if !unique.insert(id) {
                return Err(StatusCode::BAD_REQUEST);
            }
            let current = authorize_scoped(
                path,
                headers,
                scope.project.tenant_scope(),
                &scope.target_host_id,
                None,
                Capability::McpProbe(scope, id),
            )?;
            authorized = Some(current);
        }
        return authorized
            .map(|authorization: Authorization| {
                (authorization.host.credential_root, authorization.principal)
            })
            .ok_or(StatusCode::BAD_REQUEST);
    }
    authorize_scoped(
        path,
        headers,
        scope.project.tenant_scope(),
        &scope.target_host_id,
        None,
        Capability::DiagnosticProbe(scope),
    )
    .map(|authorization| (authorization.host.credential_root, authorization.principal))
}

#[cfg(target_os = "macos")]
pub(in super::super) struct PiDirectoryContext {
    pub home: PathBuf,
    pub credential_root: Option<PathBuf>,
    pub directory: Option<PathBuf>,
    pub principal: String,
}

/// Pi-specific directory permission is checked in the same grant as provider access.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_pi_directory(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    target: &str,
    write: bool,
) -> Result<PiDirectoryContext, StatusCode> {
    authorize_aggregation(
        path,
        headers,
        project,
        target,
        if write {
            Capability::AggregationWrite(project.project_id(), AppId::Pi, false)
        } else {
            Capability::AggregationRead(project.project_id(), AppId::Pi)
        },
    )
    .map(|authorization| PiDirectoryContext {
        home: authorization.host.home,
        credential_root: authorization.host.credential_root,
        directory: authorization.host.pi_directory,
        principal: authorization.principal,
    })
}

#[cfg(target_os = "macos")]
#[path = "aggregation_lifecycle_access.rs"]
mod aggregation_lifecycle;
#[cfg(target_os = "macos")]
pub(crate) use aggregation_lifecycle::{
    aggregation_lifecycle_host, authorize_aggregation_lifecycle,
};

/// Independent restart grant; configuration write never implies process control.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_codex_restart(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope,
    target: &str,
) -> Result<
    (vw_agent_apps::apps::discovery::read::LocalReadEnvironment, Option<PathBuf>, String),
    StatusCode,
> {
    let a = authorize_aggregation(
        path,
        headers,
        project,
        target,
        Capability::CodexRestart(project.project_id()),
    )?;
    Ok((
        vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: a.host.id,
            home: a.host.home,
            search_paths: a.host.search_paths,
            applications: a.host.applications,
        },
        a.host.credential_root,
        a.principal,
    ))
}

/// Session reference reads require their own exact grant, not a connection or catalog grant.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_mcp_acp_read(
    path: Option<&Path>,
    headers: &HeaderMap,
    project: &vw_api_types::ProjectScope,
    host: &str,
    session: &vw_api_types::SessionId,
) -> Result<(), StatusCode> {
    if session.as_ref().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    authorize_scoped(
        path,
        headers,
        project.tenant_scope(),
        host,
        None,
        Capability::McpAcpRead(project.project_id(), session),
    )
    .map(|_| ())
}

/// Repository grants are independent of MCP and provider configuration grants.
pub(in super::super) fn authorize_skill_repos(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str, write: bool,
) -> Result<(), StatusCode> {
    authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillRepos(scope.project_id(), write)).map(|_| ())
}

/// Separate grants for projection, marker and onboarding. No provider/plugin grant implies these.
pub(in super::super) fn authorize_claude_config(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str,
    permission: vw_api_types::agent_apps_skills::claude_api::ClaudeConfigPermission,
) -> Result<(PathBuf, PathBuf), StatusCode> {
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::ClaudeConfig(scope.project_id(), permission))?;
    let root = a.host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((a.host.home, root))
}

/// Inventory authorization resolves the SSOT exclusively from trusted host policy.
pub(in super::super) fn authorize_skill_inventory(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str,
) -> Result<PathBuf, StatusCode> {
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInventory(scope.project_id()))?;
    let root = a.host.skills_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !root.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok(root)
}

/// Installation has an independent write grant. Repository management never grants it.
pub(in super::super) fn authorize_skill_install(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str, file_id: Option<&str>,
) -> Result<(PathBuf, Option<PathBuf>), StatusCode> {
    authorize_skill_install_owner(path, headers, scope, target, file_id)
        .map(|(root, source, _)| (root, source))
}

pub(in super::super) fn authorize_skill_install_owner(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str, file_id: Option<&str>,
) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode> {
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInstall(scope.project_id()))?;
    let root = a.host.skills_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !root.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    let source = file_id.map(|id| {
        if id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let path = a.host.skill_install_files.get(id).ok_or(StatusCode::FORBIDDEN)?;
        if !path.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        Ok(path.clone())
    }).transpose()?;
    Ok((root, source, a.principal))
}

/// Unmanaged discovery and import require both inventory and install authority:
/// inventory alone must not expose unowned application contents.
pub(in super::super) fn authorize_skill_import(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str,
) -> Result<(PathBuf, PathBuf), StatusCode> {
    authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInventory(scope.project_id()))?;
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInstall(scope.project_id()))?;
    let root = a.host.skills_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !root.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok((a.host.home, root))
}

/// Job observation/cancellation shares install authority and the trusted resource binding.
pub(in super::super) fn authorize_skill_job(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str,
) -> Result<(PathBuf, String), StatusCode> {
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInstall(scope.project_id()))?;
    let root = a.host.skills_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !root.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok((root, a.principal))
}

/// Distribution requires the project install grant and uses host-owned roots only.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_skill_bindings(
    path: Option<&Path>, headers: &HeaderMap,
    scope: &vw_api_types::id::ProjectScope, target: &str,
) -> Result<(vw_agent_apps::apps::discovery::read::LocalReadEnvironment, PathBuf), StatusCode> {
    let a = authorize_scoped(path, headers, scope.tenant_scope(), target, None,
        Capability::SkillInstall(scope.project_id()))?;
    let root = a.host.skills_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !root.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok((vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
        host_id: a.host.id, home: a.host.home, search_paths: a.host.search_paths,
        applications: a.host.applications,
    }, root))
}

/// Dashboard process control requires an explicit project grant on the target host.
pub(super) fn authorize_hermes_dashboard(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::id::ProjectScope,
    target: &str,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::HermesDashboard(project.project_id()))
        .map(|a| (a.host, a.principal))
}

/// Uses the existing tenant, principal and local-host policy dispatcher.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_prompts_read(
    path: Option<&Path>, headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope, target: &str, app: AppId,
) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode> {
    let a = authorize_aggregation(path, headers, project, target,
        Capability::PromptsRead(project.project_id(), app))?;
    Ok((a.host.home, a.host.pi_directory, a.principal))
}

/// Explicit project memory grants; dashboard/provider grants do not imply access.
pub(super) fn authorize_hermes_memory(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::id::ProjectScope,
    target: &str, write: bool,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::HermesMemory(project.project_id(), write)).map(|a| a.host)
}

#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_prompts_write(
    path: Option<&Path>, headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope, target: &str, app: AppId,
) -> Result<(PathBuf, Option<PathBuf>, String), StatusCode> {
    let a = authorize_aggregation(path, headers, project, target,
        Capability::PromptsWrite(project.project_id(), app))?;
    Ok((a.host.home, a.host.pi_directory, a.principal))
}


/// Activation also needs the executor's existing transaction root.
#[cfg(target_os = "macos")]
pub(in super::super) fn authorize_prompts_activation(
    path: Option<&Path>, headers: &HeaderMap,
    project: &vw_api_types::id::ProjectScope, target: &str, app: AppId,
) -> Result<(PathBuf, Option<PathBuf>, String, PathBuf), StatusCode> {
    let a = authorize_aggregation(path, headers, project, target,
        Capability::PromptsWrite(project.project_id(), app))?;
    Ok((a.host.home, a.host.pi_directory, a.principal,
        a.host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?))
}

/// Workspace contents need explicit project grants, independent of providers.
pub(super) fn authorize_workspace(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::id::ProjectScope,
    target: &str, write: bool,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::Workspace(project.project_id(), write)).map(|a| (a.host, a.principal))
}

/// Settings grants are scoped independently from provider import and workspace IO.
pub(super) fn authorize_openclaw_settings(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::id::ProjectScope,
    target: &str, write: bool,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::OpenClawSettings(project.project_id(), write)).map(|a| a.host)
}

/// Additional sensitive permission; ordinary configuration read is insufficient.
pub(super) fn authorize_openclaw_env_read(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::id::ProjectScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::OpenClawEnvRead(project.project_id())).map(|a| a.host)
}

/// Profile access never follows implicitly from provider or workspace grants.
pub(super) fn authorize_project_profiles(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope,
    target: &str, create: bool,
) -> Result<String, StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::ProjectProfiles(project.project_id(), create)).map(|a| a.host.id)
}

/// Maintenance is an explicit shared-entity grant, independent of creation.
pub(super) fn authorize_project_profiles_maintenance(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope,
    target: &str,
) -> Result<String, StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::ProjectProfilesMaintain(project.project_id())).map(|a| a.host.id)
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalHistoryPermit {
    project_id: vw_api_types::id::ProjectId,
    provider: vw_api_types::external_session::SessionProvider,
    instance_id: String,
}
/// Administrator-owned native instance paths; never populated from request data.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ExternalHistoryInstance {
    pub instance_id: String,
    pub provider: vw_api_types::external_session::SessionProvider,
    pub root: PathBuf,
    #[serde(default)]
    pub resume: Option<ExternalResumeSettings>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ExternalResumeSettings {
    pub installation_id: vw_api_types::agent_apps::AppInstallationId,
    pub project_roots: Vec<PathBuf>,
    pub native_agent_dir: Option<PathBuf>,
    pub profile: Option<String>,
    pub session_dir: Option<String>,
    pub xdg_data_home: Option<PathBuf>,
    pub pi_config_dir: Option<PathBuf>,
    pub xdg_state_home: Option<PathBuf>,
    pub xdg_cache_home: Option<PathBuf>,
    pub agy_provider_id: Option<String>,
    pub custom_terminal: Option<ExternalCustomTerminal>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ExternalCustomTerminal {
    pub program: PathBuf,
    pub arguments: Vec<String>,
}
pub(in super::super) struct ExternalResumeHost {
    pub id: String,
    pub home: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub applications: PathBuf,
    pub credential_root: Option<PathBuf>,
}
pub(in super::super) fn authorize_external_history_resume(
    path: Option<&Path>, headers: &HeaderMap, id: &vw_api_types::external_session::ExternalSessionIdentity,
    custom: bool,
) -> Result<(ExternalResumeHost, ExternalHistoryInstance, String, bool), StatusCode> {
    let a = authorize_scoped(path, headers, id.scope.tenant_scope(), &id.target_host, None,
        Capability::ExternalHistoryResume(id.scope.project_id()))?;
    if custom {
        authorize_scoped(path, headers, id.scope.tenant_scope(), &id.target_host, None,
            Capability::ExternalTerminalSettings)?;
    }
    let mut instances = a.host.external_session_instances.iter().filter(|i|
        i.provider == id.provider && i.instance_id == id.instance_id && a.external_history.iter().any(|p|
            p.provider == i.provider && p.instance_id == i.instance_id));
    let instance = instances.next().ok_or(StatusCode::FORBIDDEN)?.clone();
    if instances.next().is_some() || !instance.root.is_absolute() || !instance.root.starts_with(&a.host.home)
        || instance.root.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::CurDir)) {
        return Err(StatusCode::FORBIDDEN);
    }
    let agy_owner = path.is_some() && a.native_login_owner(AuthService::AgyOauth) == Some(true);
    let host = ExternalResumeHost { id: a.host.id, home: a.host.home,
        search_paths: a.host.search_paths, applications: a.host.applications,
        credential_root: a.host.credential_root };
    Ok((host, instance, a.principal, agy_owner))
}
pub(in super::super) fn authorize_external_history(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, target: &str,
) -> Result<(String, PathBuf, Vec<ExternalHistoryInstance>), StatusCode> {
    let authorization = authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::ExternalHistory(project.project_id()))?;
    let permits = authorization.external_history;
    let instances: Vec<_> = authorization.host.external_session_instances.into_iter().filter(|i|
        permits.iter().any(|p| p.provider == i.provider && p.instance_id == i.instance_id)).collect();
    if instances.is_empty() { return Err(StatusCode::FORBIDDEN); }
    let mut keys = HashSet::new();
    for i in &instances {
        if i.instance_id.trim().is_empty() || !keys.insert((i.provider, &i.instance_id))
            || !i.root.is_absolute() || !i.root.starts_with(&authorization.host.home)
            || i.root.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::CurDir)) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    Ok((authorization.principal, authorization.host.home, instances))
}

/// Deletion never inherits a history read grant.
pub(in super::super) fn authorize_external_history_delete(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, target: &str,
) -> Result<(String, PathBuf, Vec<ExternalHistoryInstance>), StatusCode> {
    let authorization = authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::ExternalHistoryDelete(project.project_id()))?;
    let permits = authorization.external_history;
    let instances: Vec<_> = authorization.host.external_session_instances.into_iter().filter(|i|
        permits.iter().any(|p| p.provider == i.provider && p.instance_id == i.instance_id)).collect();
    if instances.is_empty() { return Err(StatusCode::FORBIDDEN); }
    let mut keys = HashSet::new();
    for i in &instances {
        if i.instance_id.trim().is_empty() || !keys.insert((i.provider, &i.instance_id))
            || !i.root.is_absolute() || !i.root.starts_with(&authorization.host.home)
            || i.root.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::CurDir)) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    Ok((authorization.principal, authorization.host.home, instances))
}

/// Maintenance never inherits read, deletion or provider configuration authority.
pub(in super::super) fn authorize_external_history_maintenance(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, target: &str,
) -> Result<(String, PathBuf, Vec<ExternalHistoryInstance>, PathBuf), StatusCode> {
    let authorization = authorize_scoped(path, headers, project.tenant_scope(), target, None,
        Capability::ExternalHistoryMaintenance(project.project_id()))?;
    let private = authorization.host.credential_root.clone().filter(|p| p.is_absolute()).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let permits = authorization.external_history;
    let instances: Vec<_> = authorization.host.external_session_instances.into_iter().filter(|i|
        permits.iter().any(|p| p.provider == i.provider && p.instance_id == i.instance_id)).collect();
    if instances.is_empty() { return Err(StatusCode::FORBIDDEN); }
    let mut keys = HashSet::new();
    for i in &instances {
        if i.instance_id.trim().is_empty() || !keys.insert((i.provider, &i.instance_id))
            || !i.root.is_absolute() || !i.root.starts_with(&authorization.host.home)
            || i.root.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::CurDir)) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    Ok((authorization.principal, authorization.host.home, instances, private))
}

/// Backup authority is independent of inventory, export and lifecycle grants.
pub(in super::super) fn authorize_data_backups(
    path: Option<&Path>, headers: &HeaderMap,
    target: &vw_api_types::agent_apps::data_management::DataManagementTarget,
) -> Result<PathBuf, StatusCode> {
    let authorized = authorize_scoped(path, headers, target.scope.tenant_scope(),
        &target.target_host_id, None, Capability::DataBackups(target.scope.project_id()))?;
    if authorized.principal != target.principal_id { return Err(StatusCode::FORBIDDEN); }
    authorized.host.data_backup_root.filter(|root| root.is_absolute())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

/// Resolve backup inventory ownership from authentication, never desktop claims.
pub(in super::super) fn authorize_backup_inventory(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, host: &str,
) -> Result<(PathBuf, String), StatusCode> {
    let authorized = authorize_scoped(path, headers, project.tenant_scope(), host, None,
        Capability::DataBackups(project.project_id()))?;
    let root = authorized.host.data_backup_root.filter(|root| root.is_absolute())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((root, authorized.principal))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeriodicDataBackupPermit {
    project_id: String,
    interval_hours: u32,
}

#[cfg(target_os = "macos")]
pub(in super::super) fn periodic_data_backup_targets(
    path: &Path,
) -> Result<Vec<(PathBuf, vw_agent_apps::data_management::safety_snapshot::SafetySnapshotScope, u32)>, StatusCode> {
    use vw_api_types::id::ProjectScope;
    use vw_agent_apps::data_management::safety_snapshot::SafetySnapshotScope;
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut targets = Vec::new();
    for principal in policy.principals {
        for grant in principal.grants {
            let Some(host) = policy.hosts.iter().find(|host|
                host.id == grant.host_id && host.online && matches!(host.binding, Binding::Local)
            ) else { continue };
            let Some(root) = host.data_backup_root.as_ref().filter(|root| root.is_absolute()) else { continue };
            let tenant = TenantScope::new(&grant.platform_id, &grant.company_id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            for permit in grant.periodic_data_backups {
                if permit.interval_hours == 0 { continue; }
                let project = ProjectScope::new(tenant.clone(), permit.project_id.into())
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                if !grant.data_backups.contains(project.project_id()) {
                    return Err(StatusCode::FORBIDDEN);
                }
                targets.push((root.clone(), SafetySnapshotScope {
                    project, principal_id: principal.id.clone(), host_id: host.id.clone(),
                }, permit.interval_hours));
            }
        }
    }
    Ok(targets)
}

/// Authenticate before consuming or decoding a migration body. Policy reads are
/// authentication IO only; source and target storage must remain untouched.
pub(in super::super) fn authenticate_data_migration(
    path: Option<&Path>, headers: &HeaderMap, tenant: &TenantScope,
) -> Result<(), StatusCode> {
    let digest = membership_digest(headers)?;
    let local = tenant_policy_path(tenant);
    let policy = load(path.unwrap_or(&local)).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let principal = policy.principals.iter().find(|p|
        crate::security::pairing::constant_time_eq(&p.token_sha256, &digest))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !principal.grants.iter().any(|g| g.platform_id == tenant.platform_id().as_ref()
        && g.company_id == tenant.company_id().as_ref() && !g.data_migration_read.is_empty()) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

pub(in super::super) fn authorize_data_migration(
    path: Option<&Path>, headers: &HeaderMap,
    target: &vw_api_types::agent_apps::data_management::DataManagementTarget,
    source_id: Option<&str>,
) -> Result<Option<(PathBuf, PathBuf)>, StatusCode> {
    let a = authorize_scoped(path, headers, target.scope.tenant_scope(), &target.target_host_id,
        None, Capability::DataMigrationRead(target.scope.project_id()))?;
    if a.principal != target.principal_id { return Err(StatusCode::FORBIDDEN); }
    source_id.map(|id| {
        let selected = a.host.data_migration_sources.get(id).ok_or(StatusCode::FORBIDDEN)?;
        let root = a.host.data_migration_root.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        if !root.is_absolute() || !selected.is_absolute() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        Ok((root.clone(), selected.clone()))
    }).transpose()
}

/// Explicit export grant, complete principal binding and local host ownership.
pub(in super::super) fn authorize_device_sync_upload(
    path: Option<&Path>, headers: &HeaderMap,
    target: &vw_api_types::agent_apps::DataManagementTarget,
) -> Result<(PathBuf, PathBuf), StatusCode> {
    let a = authorize_scoped(path, headers, target.scope.tenant_scope(),
        &target.target_host_id, None, Capability::DeviceSyncUpload(target.scope.project_id()))?;
    if a.principal != target.principal_id { return Err(StatusCode::FORBIDDEN); }
    let credentials = a.host.credential_root.filter(|p| p.is_absolute())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let skills = a.host.skills_root.filter(|p| p.is_absolute())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((credentials, skills))
}

/// Device-sync settings require an explicit host/project grant, independent of backups.
pub(in super::super) fn authorize_device_sync_config(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, host: &str,
) -> Result<String, StatusCode> {
    vw_agent_apps::providers::validate_identity(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    let authorized = authorize_scoped(path, headers, project.tenant_scope(), host, None,
        Capability::DeviceSyncConfig(project.project_id()))?;
    Ok(authorized.principal)
}

/// Explicit permission to send draft credentials from this local execution host.
pub(in super::super) fn authorize_device_sync_probe(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, host: &str,
) -> Result<PathBuf, StatusCode> {
    vw_agent_apps::providers::validate_identity(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    let a = authorize_scoped(path, headers, project.tenant_scope(), host, None,
        Capability::DeviceSyncProbe(project.project_id()))?;
    a.host.credential_root.filter(|p| p.is_absolute()).ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(target_os = "macos")]
pub(in super::super) fn data_location_roots(path: Option<&Path>, headers: &HeaderMap, target: &vw_api_types::agent_apps::data_management::DataManagementTarget) -> Result<(PathBuf,PathBuf,PathBuf),StatusCode> {
    let a=authorize_scoped(path,headers,target.scope.tenant_scope(),&target.target_host_id,None,Capability::DataBackups(target.scope.project_id()))?;
    if a.principal!=target.principal_id { return Err(StatusCode::FORBIDDEN); }
    let [app,device,environment]=a.host.data_location_roots.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    if ![&app,&device,&environment].iter().all(|p|p.is_absolute()) { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok((app,device,environment))
}

pub(in super::super) fn authorize_data_export(path: Option<&Path>, headers: &HeaderMap, target: &vw_api_types::agent_apps::data_management::DataManagementTarget) -> Result<(),StatusCode> {
    let a=authorize_scoped(path,headers,target.scope.tenant_scope(),&target.target_host_id,None,Capability::DataExport(target.scope.project_id()))?;
    if a.principal!=target.principal_id { return Err(StatusCode::FORBIDDEN); }
    Ok(())
}

/// Download authority is independent from export authority. Recovery stays host-local.
pub(in super::super) fn authorize_device_sync_download(
    path: Option<&Path>, headers: &HeaderMap,
    target: &vw_api_types::agent_apps::DataManagementTarget,
) -> Result<(PathBuf, PathBuf, PathBuf), StatusCode> {
    let a = authorize_scoped(path, headers, target.scope.tenant_scope(),
        &target.target_host_id, None, Capability::DeviceSyncDownload(target.scope.project_id()))?;
    if a.principal != target.principal_id { return Err(StatusCode::FORBIDDEN); }
    let credentials = a.host.credential_root.filter(|p| p.is_absolute()).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let skills = a.host.skills_root.filter(|p| p.is_absolute()).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let recovery = a.host.data_backup_root.filter(|p| p.is_absolute()).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((credentials, skills, recovery))
}

/// Background usage only consumes administrator-bound read grants on local hosts.
/// Project registration is rechecked by the worker before opening any source.
pub(in super::super) fn usage_scan_targets(path: &Path) -> Result<Vec<(vw_api_types::ProjectScope, String, PathBuf, ExternalHistoryInstance)>, StatusCode> {
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut targets = Vec::new();
    for principal in &policy.principals {
        for grant in &principal.grants {
            let Some(host) = policy.hosts.iter().find(|host| host.id == grant.host_id && host.id == vw_shared::local_apps::HOST_ID
                && host.online && matches!(host.binding, Binding::Local)) else { continue };
            let tenant = TenantScope::new(&grant.platform_id, &grant.company_id)
                .map_err(|_| StatusCode::FORBIDDEN)?;
            for permit in &grant.external_sessions_read {
                let project = vw_api_types::ProjectScope::new(tenant.clone(), permit.project_id.clone())
                    .map_err(|_| StatusCode::FORBIDDEN)?;
                for instance in host.external_session_instances.iter().filter(|instance|
                    instance.provider == permit.provider && instance.instance_id == permit.instance_id) {
                    if instance.root.is_absolute() && instance.root.starts_with(&host.home) {
                        targets.push((project.clone(), principal.id.clone(), host.home.clone(), instance.clone()));
                    }
                }
            }
        }
    }
    Ok(targets)
}

/// Ledger records combine native and relay sources, so a native instance grant
/// cannot authorize project-wide ledger queries.
pub(in super::super) fn authorize_usage_ledger_read(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, host: &str,
) -> Result<(), StatusCode> {
    authorize_scoped(path, headers, project.tenant_scope(), host, None,
        Capability::UsageLedgerRead(project.project_id()))?;
    Ok(())
}

/// Event subscriptions retain the authenticated principal for quota isolation.
pub(in super::super) fn authorize_usage_events(
    path: Option<&Path>, headers: &HeaderMap, project: &vw_api_types::ProjectScope, host: &str,
) -> Result<String, StatusCode> {
    let authorization = authorize_scoped(path, headers, project.tenant_scope(), host, None,
        Capability::UsageLedgerRead(project.project_id()))?;
    Ok(authorization.principal)
}
