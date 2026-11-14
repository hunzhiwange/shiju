use vw_gateway_client::UsageSummary;

pub struct State {
    pub display: super::display::State,
    pub pricing: super::pricing::State,
    pub scan: super::scan::State,
    pub charts: super::charts::State,
    pub logs: super::logs::State,
    pub groups: super::groups::State,
    pub query: vw_gateway_client::vw_api_types::usage::UsageQuery,
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
        self.display.close();
        self.open = false;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = false;
        self.result = None;
    }

    pub fn begin(&mut self) -> u64 {
        self.charts.loading = true;
        self.charts.result = None;
        self.groups.loading = true;
        self.groups.result = None;
        self.open = true;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = true;
        self.result = None;
        self.sequence
    }
}

impl Default for State {
    fn default() -> Self {
        Self { display: super::display::State::default(), pricing: super::pricing::State::default(), scan: super::scan::State::default(), charts: super::charts::State::default(), logs: super::logs::State::default(), groups: super::groups::State::default(), query: super::overview::default_query(), open: false, sequence: 0, loading: false, result: None }
    }
}
