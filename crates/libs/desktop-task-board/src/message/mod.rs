//! 定义任务看板消息类型，作为界面事件与状态更新之间的契约。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use desktop_task_model::{Task, TaskBoardSettings, TaskStatus};
use desktop_task_executor::executor::state::WorktreePoolSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardSettingsModalTab {
    #[default]
    Refresh,
    Scheduling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardDetailTab {
    #[default]
    Basic,
    Logs,
}
use vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue;

#[derive(Debug, Clone)]
pub struct WorktreePoolMaintenanceOutcome {
    pub idle_count: usize,
    pub target_idle_count: usize,
    pub snapshot: Option<WorktreePoolSnapshot>,
}

#[derive(Debug, Clone)]
/// 描述 TaskBoardMessage 支持的离散状态或消息分支。
pub enum TaskBoardMessage {
    Detail(crate::detail::Event),
    OpenWorkViewEditor,
    ShowWorkDisplay(bool),
    SelectWorkDisplayMode(crate::work_views::TaskBoardViewMode),
    ChangeWorkDisplay(crate::work_views::WorkDisplay),
    SearchWorkTasks(String),
    ToggleWorkDisplayGroup(crate::work_views::WorkDisplayGroup),
    WorkViewFieldChanged(u8, String),
    ToggleWorkViewLabel(String),
    WorkViewPropertyChanged(String, Option<AgentSchedulerTaskPropertyValue>),
    ApplyWorkFilter,
    ClearWorkFilter,
    OpenSavedWorkView(String),
    SaveWorkView(bool),
    DeleteWorkView,
    SelectWorkView(crate::work_views::BuiltinWorkView),
    TogglePersonalTask(String),
    OpenWorkProject(String),
    ToggleBoard,
    CloseBoard,
    LoadTasks,
    TasksLoaded {
        project_path: String,
        tasks: Vec<Task>,
    },
    CreateTask,
    CreateTaskSubmitted,
    AddTaskFromInput(String),
    AddTaskFromInputWithOptions {
        content: String,
        priority: String,
        model: String,
        reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    },
    CreateTaskCancelled,
    OpenSettingsModal,
    CloseSettingsModal,
    SelectSettingsModalTab(TaskBoardSettingsModalTab),
    UpdateDraftPriority(String),
    UpdateDraftAssignee(String),
    UpdateDraftModel(String),
    SelectDraftAgent {
        agent: String,
        acp_agent: Option<String>,
    },
    UpdateDraftReasoningEffort(Option<vw_api_chat_identity::ReasoningEffortDto>),
    UpdateDraftDescription(String),
    UpdateDraftPrompt(String),
    UpdateDraftAutoPromoteDelay(String),
    TaskCreated {
        project_path: String,
        task: Task,
    },
    TaskStatusChanged {
        task_id: String,
        new_status: TaskStatus,
    },
    TaskUpdated(Task),
    TaskDeleted(String),
    TaskArchived(String),
    ArchiveCompletedTasks,
    ToggleTaskSelection {
        task_id: String,
        selected: bool,
    },
    ToggleBulkSelectionMode(TaskStatus),
    SelectAllTasksInStatus(TaskStatus),
    InvertTaskSelectionInStatus(TaskStatus),
    UpdateBulkPriorityInput(String),
    UpdateBulkModelInput(String),
    BulkAgentSelected(String),
    UpdateBulkReasoningEffort(Option<vw_api_chat_identity::ReasoningEffortDto>),
    BulkSetPriorityInStatus(TaskStatus),
    BulkSetModelInStatus(TaskStatus),
    BulkSetAgentInStatus(TaskStatus),
    BulkSetExecutorInStatus {
        status: TaskStatus,
        executor: Option<String>,
    },
    BulkSetReasoningEffortInStatus {
        status: TaskStatus,
        reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    },
    ToggleBulkExecutorPopover,
    BulkExecutorSelected(Option<String>),
    CloseBulkExecutorPopover,
    BulkArchiveTasksInStatus(TaskStatus),
    BulkDeleteTasksInStatus(TaskStatus),
    BulkMoveTasksInStatus {
        from_status: TaskStatus,
        to_status: TaskStatus,
    },
    BulkMergeTasksInStatus(TaskStatus),
    ManualMergeQueueTick,
    BulkActionCompleted(Result<(), String>),
    MoveTaskStatusCompleted {
        target_status: TaskStatus,
        result: Result<(), String>,
    },
    DuplicateTask(String),
    DragStarted {
        task_id: String,
        from_status: TaskStatus,
    },
    DragPending {
        task_id: String,
        from_status: TaskStatus,
        press_position: iced::Point,
    },
    CardReleased {
        task_id: String,
    },
    DragEnded,
    DropOnStatus {
        to_status: TaskStatus,
        insert_index: Option<usize>,
    },
    SelectTask(String),
    DeselectTask,
    OpenTaskDetail { project_path: String, task_id: String },
    ViewTaskLogs { project_path: String, task_id: String },
    CloseTaskLogs,
    SelectDetailTab(TaskBoardDetailTab),
    EditTask(String),
    UpdateEditingTaskPriority(String),
    UpdateEditingTaskModel(String),
    UpdateEditingTaskModelInput(String),
    UpdateEditingTaskAgent(Option<String>),
    UpdateEditingTaskExecutor(Option<String>),
    UpdateEditingTaskReasoningEffort(Option<vw_api_chat_identity::ReasoningEffortDto>),
    UpdateEditingTaskPrompt(String),
    SaveEditingTask,
    ToggleEditingTaskLabel(String),
    ClearEditingTaskLabels,
    SaveEditingTaskLabels,
    EditingTaskPropertyChanged {
        task_id: String,
        field_id: String,
        value: Option<AgentSchedulerTaskPropertyValue>,
    },
    SaveEditingTaskProperty { task_id: String, field_id: String },
    EditingTaskPropertySaved {
        project_path: String,
        task_id: String,
        field_id: String,
        generation: u64,
        result: Result<Option<AgentSchedulerTaskPropertyValue>, String>,
    },
    EditingTaskLabelsSaved {
        project_path: String,
        task_id: String,
        generation: u64,
        result: Result<Vec<String>, String>,
    },
    CloseTaskPanel,
    SetMaxConcurrent(u32),
    ToggleAutoRefresh(bool),
    SetRefreshIntervalSeconds(u64),
    SetSchedulerTickIntervalSeconds(u64),
    SetAutoPromoteTickIntervalSeconds(u64),
    SetFailedRetryMinutes(u32),
    SetFailedRetryMaxAttempts(u32),
    SetRunningTimeoutMinutes(u32),
    ToggleRecycleWorktreeOnTaskFinish(bool),
    SetPrSubmittedStallTimeoutSeconds(u32),
    StartExecution,
    StopExecution,
    ExecutionTick,
    WorktreePoolMaintained {
        project_path: String,
        result: Result<WorktreePoolMaintenanceOutcome, String>,
    },
    WorktreeSnapshotLoaded {
        project_path: String,
        result: Result<Option<WorktreePoolSnapshot>, String>,
    },
    AutoMergeTick,
    UiTick,
    SimulateStep {
        task_id: String,
    },
    ExecuteTask {
        task_id: String,
    },
    TaskExecutionCompleted {
        project_path: String,
        task_id: String,
        result: Result<String, String>,
    },
    TaskMergeCompleted {
        project_path: String,
        task_id: String,
        result: Result<String, String>,
    },
    TaskResultPersistenceCompleted {
        project_path: String,
        task_id: String,
        result: Result<(), String>,
    },
    TaskRemovalWorktreeRecycled {
        project_path: String,
        task_id: String,
        result: Result<(), String>,
    },
    RecycleWorktreeSlotPressed {
        slot_id: String,
    },
    WorktreeSlotRecycled {
        slot_id: String,
        result: Result<(), String>,
    },
    ColumnScrollChanged {
        status: TaskStatus,
        has_vertical_scrollbar: bool,
    },
    FilterByStatus(Option<TaskStatus>),
    FilterByPriority(Option<(u32, u32)>),
    SortByPriority(bool),
    SortByDate(bool),
    SettingsUpdated(TaskBoardSettings),
    OpenTaskInSession {
        task_id: String,
    },
    ContextMenuOpened {
        task_id: String,
        x: f32,
        y: f32,
    },
    ContextMenuClosed,
    SetAutoPromoteDelay(u64),
    PromotePoolTasksTick,
    AgentSchedulerPoolWakeCompleted(
        Result<
            vw_api_agent_scheduler::AgentSchedulerWakeResponse,
            String,
        >,
    ),
    AgentSchedulerExecutionWakeCompleted(
        Result<
            vw_api_agent_scheduler::AgentSchedulerWakeResponse,
            String,
        >,
    ),
    AgentSchedulerCommandInFlightStatusTick {
        project_path: String,
        task_id: String,
    },
    AgentSchedulerStatusLoaded {
        project_path: String,
        generation: u64,
        result: Result<
            vw_api_agent_scheduler::AgentSchedulerTaskBoardStatusResponse,
            String,
        >,
    },
    AgentSchedulerLogsLoaded {
        project_path: String,
        generation: u64,
        requested_task_id: Option<String>,
        result: Result<
            Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>,
            String,
        >,
    },
    AgentSchedulerAutoMergeWakeCompleted(
        Result<
            vw_api_agent_scheduler::AgentSchedulerWakeResponse,
            String,
        >,
    ),
    DescEditorAction(iced::widget::text_editor::Action),
    PromptEditorAction(iced::widget::text_editor::Action),
    LogsViewerEditorAction(iced::widget::text_editor::Action),
    LogsViewerEditorWheelScrolled {
        delta: iced::mouse::ScrollDelta,
        viewport_height: f32,
    },
    LogsViewerScrollbarChanged {
        top_line: f32,
        viewport_height: f32,
    },
    LogsViewerOpenContextMenu {
        x: f32,
        y: f32,
    },
    LogsViewerCloseContextMenu,
    LogsViewerContextMenuCopy,
    LogsViewerContextMenuCut,
    LogsViewerContextMenuPaste,
    LogsViewerContextMenuDelete,
    ToggleModelPopover,
    ModelSelected(String),
    CloseModelPopover,
    ToggleBulkModelPopover,
    BulkModelSelected(String),
    CloseBulkModelPopover,
    ToggleExecutorPopover,
    CloseExecutorPopover,
    ToggleWorktreePixelOffice(bool),
    ToggleClearPromptAfterCreate(bool),
    ToggleCloseAfterCreate(bool),
    ToggleCloseAfterEdit(bool),
    ClearCreateSubmitSuccess,
    ClearEditSubmitSuccess,
    EditingTaskSaved { project_path: String, task_id: String, result: Result<(), String> },
    ToggleImportMode(bool),
    ImportEditorAction(iced::widget::text_editor::Action),
    ImportFilePick,
    ImportFilePicked(Option<String>),
    ImportFileLoaded(Result<String, String>),
    InsertDemoData(String),
    ClearImportEditor,
    ImportTasksSubmitted,
}
#[cfg(test)]
#[path = "message_tests.rs"]
mod message_tests;
