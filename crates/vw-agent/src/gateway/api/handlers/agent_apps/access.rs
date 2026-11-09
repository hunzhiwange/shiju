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
    vw_api_types::apps_usage::UsageSnapshotQuery,
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
    actions: Vec<vw_api_types::agent_apps::AppLifecycleAction>,
    #[serde(default)]
    preferences_write: bool,
    #[serde(default)]
    provider_config_read: bool,
    #[serde(default)]
    provider_config_write: bool,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    usage_refresh: Vec<UsageRefreshPermit>,
    #[serde(default)]
    usage_read: Vec<UsageReadPermit>,
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
    identities: Vec<IdentityPermit>,
    platform_id: String,
    company_id: String,
    host_id: String,
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
    pub host: Host,
    pub principal: String,
    #[cfg(target_os = "macos")]
    pub(super) usage_script_origins: Vec<reqwest::Url>,
    pub identities: Vec<IdentityPermit>,
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
    pub id: String,
    pub online: bool,
    pub binding: Binding,
    pub home: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub applications: PathBuf,
    #[serde(default)]
    pub credential_root: Option<PathBuf>,
    #[serde(default)]
    pub github_enterprise_authorities: Vec<String>,
    #[serde(default)]
    native_login_owners: Vec<NativeLoginOwner>,
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
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
    let mut values = headers.get_all("x-agent-apps-token").iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let local_path;
    let path = match path {
        Some(path) => path,
        None => {
            local_path = tenant_policy_path(tenant);
            &local_path
        }
    };
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let digest = hex::encode(Sha256::digest(token));
    let principal = policy
        .principals
        .iter()
        .find(|principal| {
            crate::security::pairing::constant_time_eq(&principal.token_sha256, &digest)
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
                    Capability::Read => true,
                    Capability::PreferencesWrite => g.preferences_write,
                    Capability::Launch => g.launch,
                    Capability::ProviderConfigRead => g.provider_config_read,
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
        host,
        principal: principal.id.clone(),
        #[cfg(target_os = "macos")]
        usage_script_origins,
        identities,
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
