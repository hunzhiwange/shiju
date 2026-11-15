//! Authorization aggregate; concrete account types belong to the account domain.
use tokio::sync::{mpsc, oneshot};
use gateway_endpoint::GatewayEndpoint;
use vw_api_app_auth::{AuthService, AuthPasteRequest};
pub use desktop_settings_account::types::*;
#[derive(Default)]
pub(crate) struct State {
    pub scope: Option<Scope>,
    pub pending: Option<u64>,
    pub rows: Option<Vec<ServiceAccounts>>,
    pub usage: crate::app::components::apps_usage_cards::Cards,
    pub loopback: bool,
    pub form: Option<Connection>,
    pub connection: Option<Connection>,
    pub connection_error: Option<String>,
    pub login: Option<Login>,
    pub binding: Option<Binding>,
    pub account_action: Option<AccountAction>,
    pub action_generation: u64,
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
            self.usage.clear();
            self.pending = None;
            self.form = None;
            self.connection = None;
            self.connection_error = None;
            self.login = None;
            self.binding = None;
            self.account_action = None;
            self.focused_service = None;
        }
    }

    pub fn rows_generation(&self) -> u64 {
        self.generation
    }

    pub fn begin(&mut self) -> u64 {
        self.usage.clear();
        self.connection_error = None;
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
