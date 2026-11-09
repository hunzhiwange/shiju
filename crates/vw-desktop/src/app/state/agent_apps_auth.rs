//! Read-only authorization center state, scoped to the active management connection.

use vw_gateway_client::{
    GatewayEndpoint,
    vw_api_types::{
        agent_apps_auth::{AuthAccount, AuthService},
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
