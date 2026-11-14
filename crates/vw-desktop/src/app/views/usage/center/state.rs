use vw_gateway_client::UsageSummary;

pub struct State {
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
        self.groups = super::groups::State::default();
        self.open = false;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = false;
        self.result = None;
    }

    pub fn begin(&mut self) -> u64 {
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
        Self { logs: super::logs::State::default(), groups: super::groups::State::default(), query: super::overview::default_query(), open: false, sequence: 0, loading: false, result: None }
    }
}
