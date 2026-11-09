//! Authorization center state, scoped to the active management connection.

use tokio::sync::oneshot;
use vw_gateway_client::{
    GatewayEndpoint,
    vw_api_types::{
        TimestampMs,
        agent_apps_auth::{
            AuthAccount, AuthLoginMethod, AuthRegion, AuthService, AuthStartRequest,
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
    pub login: Option<DeviceLogin>,
    device_generation: u64,
    generation: u64,
}

impl State {
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
pub enum DeviceTarget {
    Copilot,
    Codex,
    Xai,
    Kimi(AuthRegion),
}

impl DeviceTarget {
    pub fn service(self) -> AuthService {
        match self {
            Self::Copilot => AuthService::GithubCopilot,
            Self::Codex => AuthService::CodexOauth,
            Self::Xai => AuthService::XaiOauth,
            Self::Kimi(_) => AuthService::KimiOauth,
        }
    }

    pub fn request(self) -> AuthStartRequest {
        AuthStartRequest {
            method: AuthLoginMethod::DeviceCode,
            github_domain: None,
            region: match self {
                Self::Kimi(region) => Some(region),
                _ => None,
            },
            target_account_id: None,
        }
    }
}

pub(crate) struct DeviceLogin {
    pub generation: u64,
    pub target: DeviceTarget,
    pub phase: DevicePhase,
    // Dropping the modal signals the task even when the start response is still in flight.
    _cancel_on_close: oneshot::Sender<()>,
}

#[derive(Clone)]
pub(crate) enum DevicePhase {
    Starting,
    Pending(DeviceChallenge),
    Failed(&'static str),
}

#[derive(Clone)]
pub struct DeviceChallenge {
    pub(crate) user_code: String,
    pub(crate) verification_url: reqwest::Url,
    pub(crate) region: Option<AuthRegion>,
    pub(crate) expires_at: TimestampMs,
}

impl State {
    pub fn begin_device_login(&mut self, target: DeviceTarget) -> (u64, oneshot::Receiver<()>) {
        self.device_generation = self.device_generation.wrapping_add(1);
        let (cancel_on_close, cancelled) = oneshot::channel();
        self.login = Some(DeviceLogin {
            generation: self.device_generation,
            target,
            phase: DevicePhase::Starting,
            _cancel_on_close: cancel_on_close,
        });
        (self.device_generation, cancelled)
    }

    pub fn device_login_mut(&mut self, generation: u64) -> Option<&mut DeviceLogin> {
        self.login.as_mut().filter(|login| login.generation == generation)
    }

    pub fn is_latest_device_update(&self, scope: &Scope, generation: u64) -> bool {
        self.scope.as_ref() == Some(scope) && self.device_generation == generation
    }
}
