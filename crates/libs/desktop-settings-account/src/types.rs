//! Authorization center state, scoped to the active management connection.

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use {
    gateway_endpoint::GatewayEndpoint,
    vw_api_transport::TimestampMs,
    vw_api_app_auth::{AuthAccount, AuthCredentialState},
    vw_api_app_auth::AuthLoginMethod,
    vw_api_app_auth::AuthPasteRequest,
    vw_api_app_auth::AuthRegion,
    vw_api_app_auth::AuthService,
    vw_api_app_auth::AuthStartRequest,
    vw_api_transport::id::TenantScope,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub gateway: String,
    pub tenant: TenantScope,
}

impl From<&GatewayEndpoint> for Scope {
    fn from(endpoint: &GatewayEndpoint) -> Self {
        Self { gateway: endpoint.base_url(), tenant: endpoint.tenant_scope().clone() }
    }
}

#[derive(Clone)]
pub struct ServiceAccounts {
    pub service: AuthService,
    pub accounts: Result<Vec<AuthAccount>, &'static str>,
}

/// Explicit session credentials for a single gateway and tenant.
#[derive(Clone, Default)]
pub struct Connection {
    pub host: String,
    pub username: String,
    pub password: String,
    pub token: String,
}

impl Connection {
    pub fn local(credentials: local_app_auth::Credentials) -> Self {
        Self {
            host: local_app_auth::HOST_ID.into(),
            token: credentials.token,
            ..Self::default()
        }
    }

    pub fn valid(&self) -> bool { self.valid_management() }

    pub fn valid_management(&self) -> bool {
        !self.host.trim().is_empty() && !self.host.chars().any(char::is_control)
    }

}

#[derive(Clone, Copy)]
pub enum LoginTarget {
    Copilot,
    Codex,
    Xai,
    Kimi(AuthRegion),
    CodexBrowser,
    Claude,
    Agy,
    Dsh,
}

impl LoginTarget {
    pub fn service(self) -> AuthService {
        match self {
            Self::Copilot => AuthService::GithubCopilot,
            Self::Codex | Self::CodexBrowser => AuthService::CodexOauth,
            Self::Xai => AuthService::XaiOauth,
            Self::Kimi(_) => AuthService::KimiOauth,
            Self::Claude => AuthService::ClaudeOfficial,
            Self::Agy => AuthService::AgyOauth,
            Self::Dsh => AuthService::DshAccount,
        }
    }

    pub fn is_device(self) -> bool {
        matches!(self, Self::Copilot | Self::Codex | Self::Xai | Self::Kimi(_))
    }

    pub fn request(self, copy_link: bool) -> AuthStartRequest {
        AuthStartRequest {
            method: match self {
                Self::Claude if copy_link => AuthLoginMethod::CopyLink,
                Self::Claude | Self::CodexBrowser | Self::Agy | Self::Dsh => {
                    AuthLoginMethod::Browser
                }
                _ => AuthLoginMethod::DeviceCode,
            },
            github_domain: None,
            region: match self {
                Self::Kimi(region) => Some(region),
                _ => None,
            },
            target_account_id: None,
        }
    }
}

pub struct Login {
    pub generation: u64,
    pub target: LoginTarget,
    pub phase: LoginPhase,
    pub paste_input: String,
    pub paste_submitting: bool,
    pub paste_sender: mpsc::Sender<AuthPasteRequest>,
    // Dropping the modal signals the task even when the start response is still in flight.
    pub _cancel_on_close: oneshot::Sender<()>,
}

#[derive(Clone)]
pub enum LoginPhase {
    Starting,
    Pending(LoginChallenge),
    Failed(&'static str),
}

#[derive(Clone)]
pub struct LoginChallenge {
    pub user_code: Option<String>,
    pub verification_url: reqwest::Url,
    pub region: Option<AuthRegion>,
    pub expires_at: TimestampMs,
    pub can_paste: bool,
}

pub fn now() -> TimestampMs {
    TimestampMs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
            .unwrap_or(0),
    )
}

pub struct Binding {
    pub generation: u64,
    pub provider_generation: u64,
    pub catalog_generation: u64,
    pub provider_id: String,
    pub provider_name: String,
    pub host: String,
    pub service: AuthService,
    pub region: Option<AuthRegion>,
    pub accounts: Vec<AuthAccount>,
    pub connected: bool,
    pub selected: Option<String>,
    pub loading: bool,
    pub saving: bool,
    pub error: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccountOperation {
    SetDefault,
    Delete,
    Logout,
    Revoke,
}

pub struct AccountAction {
    pub generation: u64,
    pub operation: AccountOperation,
    pub account: AuthAccount,
    pub phase: ActionPhase,
}

#[derive(Clone)]
pub enum ActionPhase {
    Checking,
    Ready(Vec<String>),
    Blocked(&'static str),
    CheckFailed(&'static str),
    Saving,
    SaveFailed(&'static str),
}

impl ActionPhase {
    pub fn can_review(&self) -> bool {
        matches!(self, Self::CheckFailed(_))
            || matches!(self, Self::Ready(users) if !users.is_empty())
    }

    pub fn can_confirm(&self) -> bool {
        matches!(self, Self::SaveFailed(_))
            || matches!(self, Self::Ready(users) if users.is_empty())
    }
}

impl AccountOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::SetDefault => "切换默认账号",
            Self::Delete => "删除账号",
            Self::Logout => "本地登出",
            Self::Revoke => "撤销授权",
        }
    }

    pub fn success(self) -> &'static str {
        match self {
            Self::SetDefault => "已切换默认授权账号",
            Self::Delete => "已删除授权账号",
            Self::Logout => "账号已在执行主机登出",
            Self::Revoke => "上游已确认撤销授权",
        }
    }

    pub fn unavailable(self, account: &AuthAccount) -> Option<&'static str> {
        match self {
            Self::SetDefault if account.is_default => Some("此账号已经是默认账号"),
            Self::SetDefault if account.credential_state != AuthCredentialState::Ready => {
                Some("此账号需要重新授权后才能设为默认")
            }
            Self::Revoke if account.service != AuthService::DshAccount => {
                Some("此服务不支持上游撤销，可单独执行本地登出")
            }
            Self::Logout | Self::Revoke
                if account.credential_state == AuthCredentialState::SignedOut =>
            {
                Some("此账号已在执行主机登出")
            }
            _ => None,
        }
    }
}
