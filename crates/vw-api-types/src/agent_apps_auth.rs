//! Account authorization contracts shared by the Gateway and its clients.

use crate::TimestampMs;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthService {
    GithubCopilot,
    CodexOauth,
    XaiOauth,
    KimiOauth,
    ClaudeOfficial,
    AgyOauth,
    DshAccount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthLoginMethod {
    DeviceCode,
    Browser,
    CopyLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthRegion {
    MainlandCn,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthCredentialState {
    Ready,
    ReauthRequired,
    SignedOut,
}

/// Scoped account metadata projected from trusted records by the execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthAccount {
    pub account_id: String,
    pub service: AuthService,
    pub label: String,
    pub credential_state: AuthCredentialState,
    pub is_default: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authenticated_at: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_expires_at: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<AuthRegion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthStatus {
    pub service: AuthService,
    pub default_account_id: Option<String>,
    pub capabilities: AuthCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthCapabilities {
    pub login_methods: Vec<AuthLoginMethod>,
    pub can_set_default: bool,
    pub can_bind: bool,
    pub can_revoke: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthStartRequest {
    pub method: AuthLoginMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<AuthRegion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_account_id: Option<String>,
}

impl AuthStartRequest {
    /// Check the service and request shape before the host applies its domain and account policy.
    pub fn validate_for(&self, service: AuthService) -> Result<(), AuthErrorCode> {
        use AuthLoginMethod::{Browser, CopyLink, DeviceCode};
        use AuthService::{
            AgyOauth, ClaudeOfficial, CodexOauth, DshAccount, GithubCopilot, KimiOauth, XaiOauth,
        };

        match (service, self.method) {
            (GithubCopilot | XaiOauth | KimiOauth, DeviceCode)
            | (CodexOauth, DeviceCode | Browser)
            | (ClaudeOfficial, Browser | CopyLink)
            | (AgyOauth | DshAccount, Browser) => {}
            _ => return Err(AuthErrorCode::Unsupported),
        }

        if (self.github_domain.is_some() && service != GithubCopilot)
            || (self.region.is_some() && service != KimiOauth)
            || (self.target_account_id.is_some() && !matches!(service, CodexOauth | KimiOauth))
            || (service == KimiOauth && self.region.is_none() && self.target_account_id.is_none())
        {
            return Err(AuthErrorCode::InvalidInput);
        }
        Ok(())
    }
}

impl fmt::Debug for AuthStartRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthStartRequest")
            .field("method", &self.method)
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

/// Random workflow reference generated independently of upstream OAuth values.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthFlowId(pub String);

impl fmt::Debug for AuthFlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AuthFlowId([redacted])")
    }
}

/// The challenge is sent only when starting a flow. The host validates the HTTPS URL.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthStartResponse {
    Pending {
        flow_id: AuthFlowId,
        expires_at: TimestampMs,
        retry_after_seconds: u64,
        verification_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        can_paste: Option<bool>,
    },
}

impl fmt::Debug for AuthStartResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Pending { flow_id, expires_at, retry_after_seconds, can_paste, .. } = self;
        f.debug_struct("Pending")
            .field("flow_id", flow_id)
            .field("expires_at", expires_at)
            .field("retry_after_seconds", retry_after_seconds)
            .field("can_paste", can_paste)
            .finish_non_exhaustive()
    }
}

/// Polling returns a snapshot without the start challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthFlow {
    Pending { flow_id: AuthFlowId, expires_at: TimestampMs, retry_after_seconds: u64 },
    Approved { flow_id: AuthFlowId, expires_at: TimestampMs, account: AuthAccount },
    Failed { flow_id: AuthFlowId, expires_at: TimestampMs, error: AuthError },
    Expired { flow_id: AuthFlowId, expires_at: TimestampMs },
    Cancelled { flow_id: AuthFlowId, expires_at: TimestampMs },
}

/// Sensitive user input sent only to the original Claude flow.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthPasteRequest {
    pub pasted: String,
}

impl fmt::Debug for AuthPasteRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AuthPasteRequest { pasted: [redacted] }")
    }
}

/// A safe explanation prepared by the host, with no upstream error payload.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthError {
    pub code: AuthErrorCode,
    pub message: String,
    pub correlation_id: String,
}

impl fmt::Debug for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthError")
            .field("code", &self.code)
            .field("correlation_id", &self.correlation_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthErrorCode {
    InvalidInput,
    Unauthenticated,
    Forbidden,
    NotFound,
    Conflict,
    AccountInUse,
    IdentityMismatch,
    CodexOauthDuplicateAccount,
    ReauthRequired,
    CallbackUnreachable,
    Unsupported,
    ExecutionChannelUnavailable,
    UpstreamFailure,
    RemoteRevocationUnconfirmed,
    AuthorizationDenied,
    ExecutionHostUnavailable,
    KeychainUnavailable,
    StorageUnavailable,
}
