//! Authorization center state, scoped to the active management connection.

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use vw_gateway_client::{
    GatewayEndpoint,
    vw_api_types::{
        TimestampMs,
        agent_apps_auth::{
            AuthAccount, AuthLoginMethod, AuthPasteRequest, AuthRegion, AuthService,
            AuthStartRequest,
        },
        id::TenantScope,
    },
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
pub(crate) struct Connection {
    pub host: String,
    pub username: String,
    pub password: String,
    pub token: String,
}

impl Connection {
    pub fn valid(&self) -> bool {
        let username = self.username.trim();
        !self.host.trim().is_empty()
            && !self.host.chars().any(char::is_control)
            && !username.is_empty()
            && !username.contains(':')
            && !username.chars().any(char::is_control)
            && !self.password.is_empty()
            && !self.token.trim().is_empty()
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub scope: Option<Scope>,
    pub pending: Option<u64>,
    pub rows: Option<Vec<ServiceAccounts>>,
    pub loopback: bool,
    pub form: Option<Connection>,
    pub connection: Option<Connection>,
    pub login: Option<Login>,
    pub binding: Option<Binding>,
    pub focused_service: Option<AuthService>,
    pub binding_generation: u64,
    pub copy_login_link: bool,
    login_generation: u64,
    generation: u64,
}

impl State {
    pub fn with_browser_preference(open: bool) -> Self {
        Self { copy_login_link: !open, ..Self::default() }
    }

    pub fn select_endpoint(&mut self, endpoint: &GatewayEndpoint) {
        self.select(Scope::from(endpoint));
        self.loopback = endpoint.is_loopback();
    }

    pub fn select(&mut self, scope: Scope) {
        if self.scope.as_ref() != Some(&scope) {
            self.scope = Some(scope);
            self.rows = None;
            self.pending = None;
            self.form = None;
            self.connection = None;
            self.login = None;
            self.binding = None;
            self.focused_service = None;
        }
    }

    pub fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(self.generation);
        self.rows = None;
        self.generation
    }

    pub fn finish(&mut self, scope: &Scope, generation: u64, rows: Vec<ServiceAccounts>) -> bool {
        if self.scope.as_ref() != Some(scope) || self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        self.rows = Some(rows);
        true
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

pub(crate) struct Login {
    pub generation: u64,
    pub target: LoginTarget,
    pub phase: LoginPhase,
    pub paste_input: String,
    pub paste_submitting: bool,
    pub paste_sender: mpsc::Sender<AuthPasteRequest>,
    // Dropping the modal signals the task even when the start response is still in flight.
    _cancel_on_close: oneshot::Sender<()>,
}

#[derive(Clone)]
pub(crate) enum LoginPhase {
    Starting,
    Pending(LoginChallenge),
    Failed(&'static str),
}

#[derive(Clone)]
pub struct LoginChallenge {
    pub(crate) user_code: Option<String>,
    pub(crate) verification_url: reqwest::Url,
    pub(crate) region: Option<AuthRegion>,
    pub(crate) expires_at: TimestampMs,
    pub(crate) can_paste: bool,
}

pub(crate) fn now() -> TimestampMs {
    TimestampMs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
            .unwrap_or(0),
    )
}

impl State {
    pub fn begin_login(
        &mut self,
        target: LoginTarget,
    ) -> (u64, oneshot::Receiver<()>, mpsc::Receiver<AuthPasteRequest>) {
        self.login_generation = self.login_generation.wrapping_add(1);
        let (cancel_on_close, cancelled) = oneshot::channel();
        let (paste_sender, paste_receiver) = mpsc::channel(1);
        self.login = Some(Login {
            generation: self.login_generation,
            target,
            phase: LoginPhase::Starting,
            paste_input: String::new(),
            paste_submitting: false,
            paste_sender,
            _cancel_on_close: cancel_on_close,
        });
        (self.login_generation, cancelled, paste_receiver)
    }

    pub fn login_mut(&mut self, generation: u64) -> Option<&mut Login> {
        self.login.as_mut().filter(|login| login.generation == generation)
    }

    pub fn is_latest_login_update(&self, scope: &Scope, generation: u64) -> bool {
        self.scope.as_ref() == Some(scope) && self.login_generation == generation
    }
}

pub(crate) struct Binding {
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
