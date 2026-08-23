#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTaskKind {
    Shell,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundTask {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) kind: BackgroundTaskKind,
    pub(crate) title: String,
    pub(crate) status: BackgroundTaskStatus,
    pub(crate) detail: String,
    pub(crate) log: Vec<String>,
    pub(crate) supports_stop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BackgroundTaskPanel {
    pub(crate) tasks: Vec<BackgroundTask>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
}

impl BackgroundTaskPanel {
    pub(crate) fn replace(&mut self, tasks: Vec<BackgroundTask>) {
        let selected_id = self.tasks.get(self.selected).map(|task| task.id.clone());
        self.tasks = tasks;
        self.selected = selected_id
            .and_then(|id| self.tasks.iter().position(|task| task.id == id))
            .unwrap_or_default();
        self.error = None;
        self.loading = false;
    }
}
