//! 后台 shell 与智能体任务的列表视图模型。
//!
//! 这里保留用于渲染和选中的快照，不直接启动、停止或轮询后台任务。

const MAX_TASK_DETAIL_RENDER_CHARS: usize = 18_000;

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

/// Scheduler 快照保留的 canonical 状态；控制能力只能由此状态显式推导。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonicalTaskStatus {
    Draft,
    Queued,
    Assigned,
    Dispatched,
    Acknowledged,
    Active,
    Running,
    Submitted,
    Revision,
    Blocked,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Archived,
}

impl CanonicalTaskStatus {
    /// 按 canonical 状态机显式限制 TUI 可发送的任务控制。
    pub(crate) fn allows(self, action: TaskControlAction) -> bool {
        match action {
            TaskControlAction::Start => matches!(self, Self::Draft | Self::Queued),
            TaskControlAction::Retry => self == Self::Failed,
            TaskControlAction::Pause => matches!(
                self,
                Self::Assigned
                    | Self::Dispatched
                    | Self::Acknowledged
                    | Self::Active
                    | Self::Running
            ),
            TaskControlAction::Resume => self == Self::Paused,
            TaskControlAction::Cancel => matches!(
                self,
                Self::Draft
                    | Self::Queued
                    | Self::Assigned
                    | Self::Dispatched
                    | Self::Acknowledged
                    | Self::Active
                    | Self::Running
                    | Self::Submitted
                    | Self::Blocked
                    | Self::Paused
            ),
            TaskControlAction::Archive => {
                matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
            }
        }
    }
}

/// TUI 首批允许的 Scheduler 任务控制动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskControlAction {
    Start,
    Retry,
    Pause,
    Resume,
    Cancel,
    Archive,
}

impl TaskControlAction {
    pub(crate) fn command_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Retry => "retry",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Cancel => "cancel",
            Self::Archive => "archive",
        }
    }

    pub(crate) fn display_label(self) -> &'static str {
        match self {
            Self::Start => "启动",
            Self::Retry => "重试",
            Self::Pause => "暂停",
            Self::Resume => "恢复",
            Self::Cancel => "取消",
            Self::Archive => "归档",
        }
    }

    pub(crate) fn destructive(self) -> bool {
        matches!(self, Self::Cancel | Self::Archive)
    }
}

/// 一条后台任务的可显示快照，包含已收集日志和停止能力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundTask {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) kind: BackgroundTaskKind,
    pub(crate) title: String,
    pub(crate) status: BackgroundTaskStatus,
    pub(crate) canonical_status: CanonicalTaskStatus,
    pub(crate) detail: String,
    pub(crate) log: Vec<String>,
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

/// 当前项目任务在 canonical 任务图中的受限节点摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskGraphNodeSummary {
    pub(crate) status: String,
    pub(crate) readiness: String,
    pub(crate) waiting_task_ids: Vec<String>,
    pub(crate) blocking_task_ids: Vec<String>,
}

/// 可进入 transcript 的单条 Scheduler 日志摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskLogSummary {
    pub(crate) seq_no: i64,
    pub(crate) timestamp_ms: i64,
    pub(crate) transition: String,
    pub(crate) message: String,
}

/// 只保留元数据的 Scheduler 产物摘要，不携带 payload 或文件内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskArtifactSummary {
    pub(crate) run_id: Option<String>,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) path: Option<String>,
    pub(crate) created_at_ms: i64,
}

/// 三个来源相互隔离的 Scheduler 任务详情视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskDetailView {
    pub(crate) task_id: String,
    pub(crate) graph: Result<TaskGraphNodeSummary, String>,
    pub(crate) logs: Result<Vec<TaskLogSummary>, String>,
    pub(crate) artifacts: Result<Vec<TaskArtifactSummary>, String>,
}

impl TaskDetailView {
    pub(crate) fn render_text(&self) -> String {
        let graph = match &self.graph {
            Ok(node) => format!(
                "任务图：\n  状态：{}\n  就绪性：{}\n  等待：{}\n  阻塞：{}",
                display(&node.status),
                display(&node.readiness),
                display_list(&node.waiting_task_ids),
                display_list(&node.blocking_task_ids),
            ),
            Err(error) => format!("任务图：加载失败：{}", display(error)),
        };
        let logs = match &self.logs {
            Ok(logs) if logs.is_empty() => "日志：暂无日志".to_string(),
            Ok(logs) => format!(
                "日志（受限）：\n{}",
                logs.iter()
                    .map(|log| format!(
                        "  #{} · {} · {}\n    {}",
                        log.seq_no,
                        log.timestamp_ms,
                        display(&log.transition),
                        display(&log.message),
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Err(error) => format!("日志：加载失败：{}", display(error)),
        };
        let artifacts = match &self.artifacts {
            Ok(artifacts) if artifacts.is_empty() => "产物：暂无产物".to_string(),
            Ok(artifacts) => format!(
                "产物（仅元数据）：\n{}",
                artifacts
                    .iter()
                    .map(|artifact| format!(
                        "  {} · {} · run {} · {}\n    路径（仅元数据）：{}",
                        display(&artifact.kind),
                        display(&artifact.name),
                        artifact.run_id.as_deref().map(display).unwrap_or("—"),
                        artifact.created_at_ms,
                        artifact.path.as_deref().map(display).unwrap_or("—"),
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Err(error) => format!("产物：加载失败：{}", display(error)),
        };
        let rendered = format!(
            "Scheduler 任务详情\n任务 ID：{}\n{}\n{}\n{}",
            display(&self.task_id),
            graph,
            logs,
            artifacts,
        );
        if rendered.chars().count() <= MAX_TASK_DETAIL_RENDER_CHARS {
            rendered
        } else {
            format!(
                "{}...",
                rendered
                    .chars()
                    .take(MAX_TASK_DETAIL_RENDER_CHARS.saturating_sub(3))
                    .collect::<String>()
            )
        }
    }
}

fn display(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() { "—".to_string() } else { values.join(", ") }
}
