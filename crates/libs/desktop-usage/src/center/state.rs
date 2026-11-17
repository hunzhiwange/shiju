use gateway_usage_read::UsageSummary;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DetailTab { #[default] Logs, Providers, Models }

pub struct State {
    pub today: Option<(crate::Identity, UsageSummary)>,
    pub today_sequence: u64,
    pub sync_requested: bool,
    pub tab: DetailTab,
    pub hero: super::hero::State,
    pub range_picker: super::range_picker::State,
    pub(super) scope: Option<crate::Identity>,
    pub(super) activity: Option<(crate::Identity, bool)>,
    pub(super) activity_client: Option<vw_gateway_client::GatewayClient>,
    pub(super) activity_pending: bool,
    pub(super) status_pending: bool,
    pub(super) last_status: Option<std::time::Instant>,
    pub(super) last_query: Option<std::time::Instant>,
    pub display: super::display::State,
    pub models: crate::models::State,
    pub pricing: super::pricing::State,
    pub scan: super::scan::State,
    pub charts: super::charts::State,
    pub logs: super::logs::State,
    pub groups: super::groups::State,
    pub query: vw_api_usage::UsageQuery,
    pub open: bool,
    pub sequence: u64,
    pub loading: bool,
    pub result: Option<Result<UsageSummary, String>>,
}

impl State {
    pub fn close(&mut self) {
        self.charts = super::charts::State::default();
        self.groups = super::groups::State::default();
        self.pricing.close();
        self.models.close();
        self.display.close();
        self.status_pending = false;
        self.open = false;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = false;
        self.result = None;
        self.scan.open = false;
        self.sync_requested = false;
    }

    pub fn begin(&mut self) -> u64 {
        self.charts.loading = true;
        self.charts.result = None;
        self.groups.loading = true;
        self.groups.result = None;
        self.last_query = Some(std::time::Instant::now());
        self.open = true;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = true;
        self.result = None;
        self.sequence
    }
}

impl Default for State {
    fn default() -> Self {
        Self { today: None, today_sequence: 0, sync_requested: false, tab: DetailTab::default(), hero: super::hero::State::default(), range_picker: super::range_picker::State::default(), scope: None, activity: None, activity_client: None, activity_pending: false, status_pending: false, last_status: None, last_query: None, display: super::display::State::default(), models: crate::models::State::default(), pricing: super::pricing::State::default(), scan: super::scan::State::default(), charts: super::charts::State::default(), logs: super::logs::State::default(), groups: super::groups::State::default(), query: super::overview::default_query(), open: false, sequence: 0, loading: false, result: None }
    }
}

impl State {
    pub fn accept(&mut self, current: Option<&crate::Identity>, visible: bool, ticket: u64,
        scope: &crate::Identity, result: Result<UsageSummary, String>) {
        if self.open && self.loading && self.sequence == ticket && current == Some(scope) && visible {
            self.loading = false;
            self.result = Some(result);
        }
    }
    pub fn missing_scope(&mut self) {
        self.charts.loading = false;
        self.charts.result = Some(Err("网关不可用，请检查连接".into()));
        self.groups.loading = false;
        self.groups.result = Some(Err("网关不可用，请检查连接".into()));
        self.loading = false;
        self.result = Some(Err("网关不可用，请检查连接".into()));
    }
}

impl State {
    pub fn filter(&mut self, change: super::overview::Change) {
        super::overview::apply(&mut self.query, change);
    }
    pub fn range(&mut self, all: bool) {
        self.query.range = if all { vw_api_usage::UsageRange::All }
            else { vw_api_usage::UsageRange::Rolling { seconds: 86400 } };
    }
}
