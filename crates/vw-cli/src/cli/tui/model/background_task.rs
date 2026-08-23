//! 后台 shell 与智能体任务的列表视图模型。
//!
//! 这里保留用于渲染和选中的快照，不直接启动、停止或轮询后台任务。

/// 后台任务的执行载体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTaskKind {
    Shell,
    Agent,
}

/// 后台任务在面板中显示的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 一条后台任务的可显示快照，包含已收集日志和停止能力。
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

/// 后台任务面板的列表、选中项与加载状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BackgroundTaskPanel {
    pub(crate) tasks: Vec<BackgroundTask>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
    pub(crate) loading: bool,
}

impl BackgroundTaskPanel {
    /// 替换列表时尽量按任务 ID 保留选中项，并结束加载状态。
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
