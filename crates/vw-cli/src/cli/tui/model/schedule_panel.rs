#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchedulePanelItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) schedule: String,
    pub(crate) next_run: String,
    pub(crate) last_status: Option<String>,
    pub(crate) enabled: bool,
    pub(crate) history: Vec<String>,
    pub(crate) history_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SchedulePanel {
    pub(crate) items: Vec<SchedulePanelItem>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
}
