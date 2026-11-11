//! Mode transaction owner. Runtime integration is injected by the trusted executor.
use super::{
    claude::ClaudeFile,
    live::{commit::*, write_guard::WriteError},
};
use serde_json::Value;
use vw_api_types::{agent_apps::AppId, app_providers::mode_apply::*, id::TenantScope};

/// Observe the existing mode transaction before any route preparation. Recovery
/// and configuration writes remain owned by the mode publication path.
pub fn observe_claude_route(
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
) -> Result<AppState, ModeApplyError> {
    let state = transactions.state(AppId::Claude)?;
    super::mode_state::observe(&state, tenant).map_err(|_| ModeApplyError::Ownership)?;
    transactions.claude_directory(tenant)?;
    if let Some(pending) = transactions.pending(AppId::Claude)? {
        if pending.tenant != *tenant {
            return Err(ModeApplyError::Ownership);
        }
        return Err(WriteError::Conflict.into());
    }
    if state.stack.enabled {
        return Err(ModeApplyError::InvalidInput);
    }
    Ok(state)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeApplyError {
    Unavailable,
    InvalidInput,
    Ownership,
    Write(WriteError),
}
impl From<WriteError> for ModeApplyError {
    fn from(e: WriteError) -> Self {
        Self::Write(e)
    }
}
/// Frozen upstream client placeholder, not an upstream credential or session token.
pub const CLIENT_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";
/// Scope is executor-authorized. It is never accepted from a runtime or HTTP body.
pub struct RouteScope<'a> {
    pub tenant: &'a TenantScope,
    pub host: &'a str,
    pub app: AppId,
}
/// A readiness result contains only the trusted local client address. No upstream secret.
pub struct ClientEndpoint {
    pub(crate) url: String,
}
impl ClientEndpoint {
    pub fn new(url: &str) -> Result<Self, ModeApplyError> {
        let parsed = reqwest::Url::parse(url).map_err(|_| ModeApplyError::InvalidInput)?;
        if parsed.scheme() != "http"
            || !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
            || parsed.port().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(ModeApplyError::InvalidInput);
        }
        Ok(Self { url: parsed.as_str().trim_end_matches('/').into() })
    }
}
/// Sole integration point for mother-8's runtime. Implementations must bound I/O,
/// prove readiness for this scope and publish targets idempotently. Preparation
/// is read-only and may be probed before locking; it is repeated under the guard.
/// Publication holds the guard; neither method may reenter configuration writes.
/// No default/empty implementation or implicit listener is installed.
pub trait RouteRuntime: Send + Sync {
    fn prepare(&self, scope: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError>;
    fn publish(&self, scope: &RouteScope<'_>, provider: Option<&str>)
    -> Result<(), ModeApplyError>;
}
pub struct ClaudeMode<'a> {
    pub file: &'a mut ClaudeFile,
    pub transactions: &'a ProviderTransactions,
    pub scope: RouteScope<'a>,
    pub runtime: Option<&'a dyn RouteRuntime>,
}
impl ClaudeMode<'_> {
    /// Rows and their preimages are captured by the executor; validate runs again
    /// immediately before publication. The direct pointer is never changed here.
    pub fn apply(
        &mut self,
        action: &ModeApplyAction,
        settings: &Value,
        validate: impl Fn() -> bool,
    ) -> Result<ModeApplyResult, ModeApplyError> {
        use super::{
            live::{stage::FileChange, write_guard::FilePrivacy},
            mode_state::{self, ClientConfiguration},
        };
        use serde_json::json;
        use std::ffi::OsStr;
        use vw_api_types::app_providers::mode_state::{ProviderMode, RewriteDecision};
        if self.scope.app != AppId::Claude {
            return Err(ModeApplyError::InvalidInput);
        }
        if self.transactions.execution_host() != &ExecutionHostId::new(self.scope.host)? {
            return Err(ModeApplyError::Ownership);
        }
        // Check ownership before recovering any previously published intent.
        let state = self.transactions.state(AppId::Claude)?;
        mode_state::observe(&state, self.scope.tenant).map_err(|_| ModeApplyError::Ownership)?;
        if self
            .transactions
            .pending(AppId::Claude)?
            .as_ref()
            .is_some_and(|p| p.tenant != *self.scope.tenant)
        {
            return Err(ModeApplyError::Ownership);
        }
        if super::claude_login::configuration_has_pending_login(&self.file.guard)? {
            return Err(WriteError::Conflict.into());
        }
        if self.transactions.recover(&mut self.file.guard)?.is_some() {
            return Err(WriteError::Recovered.into());
        }
        // Stack projection/catalog policy belongs to mother-10. Never silently
        // discard its members or pass an ordinary route off as a stack contract.
        if state.stack.enabled && matches!(action, ModeApplyAction::Proxy { .. }) {
            return Err(ModeApplyError::InvalidInput);
        }
        let proxy = matches!(action, ModeApplyAction::Proxy { .. });
        let mut target = settings.clone();
        if !target.is_object() || !target.get("env").is_some_and(Value::is_object) {
            return Err(ModeApplyError::InvalidInput);
        }
        if let ModeApplyAction::Proxy { provider_id } = action {
            super::validate_identity(provider_id).map_err(|_| ModeApplyError::InvalidInput)?;
            let endpoint = self.runtime.ok_or(ModeApplyError::Unavailable)?.prepare(&self.scope)?;
            let env = target["env"].as_object().ok_or(ModeApplyError::InvalidInput)?;
            // Keep client-owned compatibility values and model fields; eliminate
            // credentials/protocol selectors that would bypass the local runtime.
            let mut routed = serde_json::Map::new();
            for (key, value) in env {
                if super::projection::fields::claude_exclusive_env(key)
                    || key == "ANTHROPIC_MODEL"
                    || key.starts_with("ANTHROPIC_DEFAULT_")
                {
                    routed.insert(key.clone(), value.clone());
                }
            }
            routed.insert("ANTHROPIC_BASE_URL".into(), json!(endpoint.url));
            routed.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(CLIENT_TOKEN_PLACEHOLDER));
            target = json!({"env":routed});
        }
        let configuration = ClientConfiguration::saved(AppId::Claude, &target, false)
            .map_err(|_| ModeApplyError::InvalidInput)?;
        let rewrite = mode_state::rewrite_decision(
            &state,
            if proxy { ProviderMode::Proxy } else { ProviderMode::Direct },
            &configuration,
        ) != RewriteDecision::SkipRewrite;
        let contract = configuration.contract();
        // A detach keeps the logical proxy mode, route and stack, but its recorded
        // client contract is now direct; attached=false forces the next attach.
        let next = ModeState {
            mode: if proxy
                || matches!(action, ModeApplyAction::Detach {})
                    && state.mode.as_ref().is_some_and(|m| m.mode == Mode::Proxy)
            {
                Mode::Proxy
            } else {
                Mode::Direct
            },
            attached: proxy,
            proxy_route: match action {
                ModeApplyAction::Proxy { provider_id } => Some(provider_id.clone()),
                _ => state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
            },
            contract: Some(contract),
        };
        let previous = state
            .mode
            .as_ref()
            .and_then(|m| m.contract.as_ref())
            .map(|c| json!({"env":c.exclusive}));
        let changed = std::cell::Cell::new(false);
        let change = |pre: Option<&[u8]>| {
            let after = super::claude::patch_with_residue(pre, previous.as_ref(), &target, false)?;
            // The transaction may replan against an external edit; report its last
            // projection instead of a stale preflight byte comparison.
            changed.set(Some(after.as_slice()) != pre);
            Ok(after)
        };
        let changes = if rewrite {
            vec![FileChange {
                name: OsStr::new(self.file.name),
                privacy: FilePrivacy::Private,
                patch: &change,
            }]
        } else {
            vec![]
        };
        self.transactions.commit_checked(
            self.scope.tenant,
            &mut self.file.guard,
            &changes,
            PendingTarget { mode: Some(next), ..Default::default() },
            || {
                if validate() && self.transactions.state(AppId::Claude)? == state {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )?;
        // File commit and durable target always precede external publication. If
        // publication fails, a repeated operation recomputes the same contract,
        // skips the files and retries this idempotent call under the same guard.
        let publication = match self.runtime {
            Some(runtime) => {
                if runtime
                    .publish(
                        &self.scope,
                        if let ModeApplyAction::Proxy { provider_id } = action {
                            Some(provider_id)
                        } else {
                            None
                        },
                    )
                    .is_ok()
                {
                    TargetPublication::Published
                } else {
                    TargetPublication::Pending
                }
            }
            None => TargetPublication::Unavailable,
        };
        Ok(ModeApplyResult {
            configuration_changed: changed.get(),
            attached: proxy,
            target_publication: publication,
        })
    }
}
