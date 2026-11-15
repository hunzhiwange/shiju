pub mod types;
pub mod system_settings_auth;
use types::*;
/// Local account interactions; `U` carries messages from injected usage cards.
#[derive(Clone)]
pub enum Event<U = ()> {
    Usage(U),
    Open,
    Close,
    Host(String),
    Username(String),
    Password(String),
    Token(String),
    Connect,
    Refresh,
    RefreshUsage,
    RetryService(vw_api_app_auth::AuthService),
    OpenBrowserPreference(bool),
    StartLogin(LoginTarget),
    RetryLogin,
    CloseLogin,
    LoginLink(u64, LinkAction),
    PasteInput(u64, String),
    PasteSubmit(u64),
    CloseBinding(u64),
    SelectBinding(u64, String),
    LoadBinding(u64),
    SaveBinding(u64),
    AccountAction(ActionEvent),
}

#[derive(Clone)]
pub enum ActionEvent {
    Open {
        rows_generation: u64,
        operation: AccountOperation,
        service: vw_api_app_auth::AuthService,
        account_id: String,
    },
    Review(u64),
    Confirm(u64),
    Close(u64),
}

#[derive(Clone, Copy)]
pub enum LinkAction {
    Open,
    Copy,
}

/// Borrowed account fields used by the view; no global application access.
pub struct State<'a> {
    pub scope: &'a Option<Scope>,
    pub rows: &'a Option<Vec<ServiceAccounts>>,
    pub form: &'a Option<Connection>,
    pub connection: &'a Option<Connection>,
    pub connection_error: &'a Option<String>,
    pub login: &'a Option<Login>,
    pub binding: &'a Option<Binding>,
    pub account_action: &'a Option<AccountAction>,
    pub pending: Option<u64>,
    pub loopback: bool,
    pub focused_service: Option<vw_api_app_auth::AuthService>,
    pub copy_login_link: bool,
    pub rows_generation: u64,
}
pub struct Usage<'a, U: Clone> {
    pub available: bool,
    pub batch_pending: bool,
    pub lines: Vec<String>,
    pub accounts: std::collections::HashMap<(vw_api_app_auth::AuthService, String), iced::Element<'a, Event<U>>>,
}
