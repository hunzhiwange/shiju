use vw_gateway_client::UsageSummary;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub sequence: u64,
    pub loading: bool,
    pub result: Option<Result<UsageSummary, String>>,
}

impl State {
    pub fn close(&mut self) {
        self.open = false;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = false;
        self.result = None;
    }

    pub fn begin(&mut self) -> u64 {
        self.open = true;
        self.sequence = self.sequence.wrapping_add(1);
        self.loading = true;
        self.result = None;
        self.sequence
    }
}
