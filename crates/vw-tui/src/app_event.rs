//! 用于协调界面动作的应用级事件。
//!
//! `AppEvent` 是界面组件与顶层 `App` 循环之间的内部消息总线。组件借此请求必须由
//! 应用层处理的动作，例如打开选择器、持久化配置或关闭代理，而无需直接访问 `App` 内部。
//!
//! 退出通过 `AppEvent::Exit(ExitMode)` 显式建模，使调用方可以请求先关闭再退出，
//! 而不必耦合应用循环中的关闭与退出时序。
//! Application-level events used to coordinate UI actions.
//!
//! `AppEvent` is the internal message bus between UI components and the top-level `App` loop.
//! Widgets emit events to request actions that must be handled at the app layer (like opening
//! pickers, persisting configuration, or shutting down the agent), without needing direct access to
//! `App` internals.
//!
//! Exit is modelled explicitly via `AppEvent::Exit(ExitMode)` so callers can request shutdown-first
//! quits without reaching into the app loop or coupling to shutdown/exit sequencing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::inline_visualization::InlineVisualizationContext;
use codex_app_server_protocol::AddCreditsNudgeCreditType;
use codex_app_server_protocol::AddCreditsNudgeEmailStatus;
use codex_app_server_protocol::ConsumeAccountRateLimitResetCreditResponse;
use codex_app_server_protocol::GetAccountRateLimitsResponse;
use codex_app_server_protocol::GetAccountTokenUsageResponse;
use codex_app_server_protocol::MarketplaceAddResponse;
use codex_app_server_protocol::MarketplaceRemoveResponse;
use codex_app_server_protocol::MarketplaceUpgradeResponse;
use codex_app_server_protocol::McpServerStatus;
use codex_app_server_protocol::McpServerStatusDetail;
use codex_app_server_protocol::PluginInstallResponse;
use codex_app_server_protocol::PluginListResponse;
use codex_app_server_protocol::PluginMarketplaceEntry;
use codex_app_server_protocol::PluginReadParams;
use codex_app_server_protocol::PluginReadResponse;
use codex_app_server_protocol::PluginUninstallResponse;
use codex_app_server_protocol::SkillsListResponse;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadGoalStatus;
use codex_app_server_protocol::ThreadItemsListResponse;
use codex_connectors::AppInfo;
use codex_file_search::FileMatch;
use codex_message_history::HistoryBatchCursor;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ModelPreset;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_approval_presets::ApprovalPreset;
use strum_macros::IntoStaticStr;
use uuid::Uuid;

use crate::app_command::AppCommand;
use crate::app_server_session::AppServerStartedThread;
use crate::bottom_pane::ApprovalRequest;
use crate::bottom_pane::StatusLineItem;
use crate::bottom_pane::TerminalTitleItem;
use crate::chatwidget::ConnectorScopeGeneration;
use crate::chatwidget::ThreadUsageOutcome;
use crate::chatwidget::UserMessage;
use crate::goal_files::GoalDraft;
use codex_app_server_protocol::AskForApproval;
use codex_config::types::ApprovalsReviewer;
use codex_features::Feature;
use codex_plugin::PluginCapabilitySummary;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::Personality;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;

use crate::history_cell::HistoryCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreadGoalSetMode {
    ConfirmIfExists,
    ReplaceExisting,
    UpdateExisting {
        status: ThreadGoalStatus,
        token_budget: Option<i64>,
    },
}

/// 批量查询返回的一个绝对历史偏移；畸形行以 `entry: None` 保留偏移，
/// 使输入框能缓存缺口而不移动所有更旧记录。
/// One absolute history offset returned by a batch lookup.
///
/// Malformed rows retain their offset with `entry` set to `None` so the composer can cache the gap
/// without shifting every older record.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HistoryBatchEntryResponse {
    pub(crate) offset: usize,
    pub(crate) entry: Option<String>,
}

/// 路由回发起请求线程的持久历史数据；批量响应保留绝对偏移和畸形行缺口，
/// 因而可独立于响应到达时的活动搜索词进行缓存。
/// Persistent-history data routed back to the thread that requested it.
///
/// Batch responses preserve absolute offsets and malformed-row gaps so the composer can cache the
/// data independently of whichever search query is active when the response arrives.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HistoryLookupResponse {
    Entry {
        offset: usize,
        log_id: u64,
        entry: Option<String>,
    },
    Batch {
        cursor: HistoryBatchCursor,
        log_id: u64,
        entries: Vec<HistoryBatchEntryResponse>,
        next_older_cursor: Option<HistoryBatchCursor>,
    },
    BatchError {
        cursor: HistoryBatchCursor,
        log_id: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsolidationScrollbackReflow {
    IfResizeReflowRan,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) enum WindowsSandboxEnableMode {
    Elevated,
    Legacy,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) struct ConnectorsSnapshot {
    pub(crate) connectors: Vec<AppInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginLocation {
    Local { marketplace_path: AbsolutePathBuf },
    Remote { marketplace_name: String },
}

impl PluginLocation {
    pub(crate) fn into_request_params(self) -> (Option<AbsolutePathBuf>, Option<String>) {
        match self {
            PluginLocation::Local { marketplace_path } => (Some(marketplace_path), None),
            PluginLocation::Remote { marketplace_name } => (None, Some(marketplace_name)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginRemoteSectionError {
    pub(crate) section_id: String,
    pub(crate) label: String,
    pub(crate) message: String,
}

/// 区分限额刷新来源，使完成处理器能正确路由结果：启动预取只更新缓存，
/// `/status` 需结束对应刷新卡片，`/usage` 可更新零余额菜单，重置选择器则需先刷新明细。
/// Distinguishes why a rate-limit refresh was requested so the completion
/// handler can route the result correctly.
///
/// A `StartupPrefetch` fires once, concurrently with the rest of TUI init, and
/// updates the cached snapshots and any available reset-credit notice (no
/// status card to finalize). A `StatusCommand` is tied to a specific `/status`
/// invocation and must call `finish_status_rate_limit_refresh` when done so the
/// card stops showing a "refreshing" state. A `UsageMenu` refreshes a cached
/// zero reset count so the disabled menu entry can become available without a
/// restart. A `ResetPicker` refreshes the rate limits and detailed reset-credit
/// rows before showing redemption choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RateLimitRefreshOrigin {
    /// 启动后主动预取 `/status` 数据和重置额度可用性。
    /// Eagerly fetched after bootstrap for `/status` data and reset availability.
    StartupPrefetch { reset_hint_request_id: u64 },
    /// 用户通过 `/status` 发起，`request_id` 用于关联待更新的状态卡片。
    /// User-initiated via `/status`; the `request_id` correlates with the
    /// status card that should be updated when the fetch completes.
    StatusCommand { request_id: u64 },
    /// 缓存重置额度为零时用户重新打开了 `/usage`。
    /// User reopened `/usage` while the cached reset-credit count was zero.
    UsageMenu { request_id: u64 },
    /// 用户打开了重置额度选择器。
    /// User opened the reset-credit picker.
    ResetPicker { request_id: u64 },
    /// 成功消费重置额度后请求刷新。
    /// Refresh requested after a reset credit was successfully consumed.
    ResetConsume { request_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeymapEditIntent {
    ReplaceAll,
    AddAlternate,
    ReplaceOne { old_key: String },
}

/// 一次 `/keymap` 捕获所记录的按键数量模式。
/// Number of key strokes recorded by one `/keymap` capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeymapCaptureMode {
    SingleKey,
    Chord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TranscriptExportDestination {
    Clipboard,
    File(PathBuf),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, IntoStaticStr)]
pub(crate) enum AppEvent {
    /// 打开守护进程范围内已加载根会话的总览。
    /// Open the daemon-wide overview of loaded root sessions.
    OpenAgentsOverview,
    /// 后台线程列表完成后更新守护进程总览。
    /// Update the daemon-wide overview after a background thread listing finishes.
    AgentsOverviewThreadsLoaded {
        request_id: Uuid,
        result: Result<Vec<Thread>, String>,
    },
    /// 切换到共享仪表盘中选定的根会话。
    /// Switch to a root session selected from the shared dashboard.
    SelectAgentsOverviewThread {
        thread_id: ThreadId,
    },
    /// 直接从共享仪表盘启动后台任务。
    /// Start a background task directly from the shared dashboard.
    DispatchAgentsOverviewTask {
        prompt: String,
        cwd: Option<AbsolutePathBuf>,
    },
    /// 直接从共享仪表盘重命名任务。
    /// Rename a task directly from the shared dashboard.
    RenameAgentsOverviewThread {
        thread_id: ThreadId,
        name: String,
    },
    /// 直接从共享仪表盘中断任务。
    /// Interrupt a task directly from the shared dashboard.
    StopAgentsOverviewThread {
        thread_id: ThreadId,
    },
    /// 启动共享应用服务器守护进程，但不迁移当前嵌入式会话。
    /// Start the shared app-server daemon without moving the current embedded session.
    #[cfg(unix)]
    StartAgentsDaemon,
    /// 报告共享应用服务器守护进程是否启动成功。
    /// Report whether starting the shared app-server daemon succeeded.
    #[cfg(unix)]
    AgentsDaemonStarted {
        result: Result<(), String>,
    },
    /// 打开用于切换活动线程的代理选择器。
    /// Open the agent picker for switching active threads.
    OpenAgentPicker,
    /// 合并已完成的根范围代理列表刷新，而不阻塞终端输入。
    /// Merge a completed root-scoped agent-picker refresh without blocking terminal input.
    AgentPickerThreadsLoaded {
        primary_thread_id: ThreadId,
        request_id: Uuid,
        result: Result<Vec<Thread>, String>,
    },
    /// 把活动线程切换到选定代理。
    /// Switch the active thread to the selected agent.
    SelectAgentThread(ThreadId),

    /// 将当前线程派生为临时侧会话。
    /// Fork the current thread into a transient side conversation.
    StartSide {
        parent_thread_id: ThreadId,
        user_message: Option<UserMessage>,
    },

    /// 无论当前焦点如何，都向指定线程提交操作。
    /// Submit an op to the specified thread, regardless of current focus.
    SubmitThreadOp {
        thread_id: ThreadId,
        op: AppCommand,
    },

    /// 中断并派生安全缓冲轮次，再使用服务器选择的模型重试。
    /// Interrupt, fork, and retry a safety-buffered turn with the server-selected model.
    RetrySafetyBufferedTurn {
        thread_id: ThreadId,
        turn_id: String,
        model: String,
        turn: AppCommand,
        prompt: UserMessage,
    },

    /// 向指定线程通道投递合成的历史查询响应。
    /// Deliver a synthetic history lookup response to a specific thread channel.
    ThreadHistoryEntryResponse {
        thread_id: ThreadId,
        event: HistoryLookupResponse,
    },

    /// 分页历史行重排后，用更旧历史补充终端回滚区。
    /// Refill terminal scrollback from older paginated history after its rows reflow.
    RequestOlderScrollbackHistory {
        thread_id: ThreadId,
    },

    /// 后台加载的一页较旧 Ctrl+T 记录历史。
    /// One background-loaded page of older Ctrl+T transcript history.
    OlderThreadHistoryLoaded {
        thread_id: ThreadId,
        cursor: String,
        result: Result<ThreadItemsListResponse, String>,
    },

    /// 为按需导出 Markdown 记录打开文件名提示。
    /// Open the filename prompt for an on-demand Markdown transcript export.
    OpenTranscriptExportFilePrompt,

    /// 把当前线程的全部历史导出到选定目标。
    /// Export all current-thread history to the selected destination.
    ExportTranscript {
        destination: TranscriptExportDestination,
    },

    /// 复制选择器结果，并在聊天组件中保留剪贴板租约。
    /// Copy a picker selection while retaining its clipboard lease in the chat widget.
    CopySelection {
        text: Arc<str>,
        label: String,
    },

    /// 将已提交提示持久化到跨会话消息历史。
    /// Persist a submitted prompt in the cross-session message history.
    AppendMessageHistoryEntry {
        thread_id: ThreadId,
        text: String,
    },

    /// 把应用 Git 动作指令发现的分支持久化到线程元数据。
    /// Persist a branch discovered from an App git-action directive into thread metadata.
    SyncThreadGitBranch {
        thread_id: ThreadId,
        branch: String,
        cwd: PathBuf,
    },

    /// 按偏移获取一条持久化跨会话消息历史。
    /// Fetch a persistent cross-session message history entry by offset.
    LookupMessageHistoryEntry {
        thread_id: ThreadId,
        offset: usize,
        log_id: u64,
    },

    /// 获取一批有界持久历史项供反向搜索。
    /// Fetch a bounded batch of persistent history entries for reverse search.
    LookupMessageHistoryBatch {
        thread_id: ThreadId,
        cursor: HistoryBatchCursor,
        log_id: u64,
    },

    /// 启动新会话，并可选指定名称。
    /// Start a new session, optionally assigning it a name.
    NewSession {
        name: Option<String>,
    },

    /// 修改发起事件的空闲主线程工作目录。
    /// Change the working directory of the originating idle primary thread.
    ChangeWorkingDirectory {
        thread_id: ThreadId,
        requested_cwd: PathBuf,
    },

    /// 输入界面可用后挂接的新启动线程结果。
    /// Result of the fresh startup thread that is attached after the input UI is live.
    StartupThreadStarted {
        result: Result<AppServerStartedThread, String>,
    },

    /// 清除终端界面（屏幕及回滚区），启动新会话，并保留旧聊天以便恢复。
    /// Clear the terminal UI (screen + scrollback), start a fresh session, and keep the
    /// previous chat resumable.
    ClearUi {
        name: Option<String>,
    },

    /// 使用选定的回滚区渲染模式重新渲染记录。
    /// Re-render the transcript using the selected scrollback rendering mode.
    RawOutputModeChanged {
        enabled: bool,
    },

    /// 清除当前上下文、启动新会话并提交初始用户消息。
    /// 这是计划模式交接路径：旧线程仍可恢复，但新会话配置完成后模型只看到 `text` 中的显式提示。
    /// Clear the current context, start a fresh session, and submit an initial user message.
    ///
    /// This is the Plan Mode handoff path: the previous thread remains resumable, but the model
    /// sees only the explicit prompt carried in `text` once the new session is configured.
    ClearUiAndSubmitUserMessage {
        text: String,
    },

    /// 在运行中的 TUI 会话内打开恢复选择器。
    /// Open the resume picker inside the running TUI session.
    OpenResumePicker,

    /// 在运行中的 TUI 会话内打开 Claude Code 迁移选择器。
    /// Open the Claude Code migration picker inside the running TUI session.
    OpenExternalAgentConfigMigration,

    /// 在运行中的 TUI 会话内按 UUID 或线程名恢复线程。
    /// Resume a thread by UUID or thread name inside the running TUI session.
    ResumeSessionByIdOrName(String),

    /// 归档当前活动主线程，并在成功后退出。
    /// Archive the current active main thread and exit after it succeeds.
    ArchiveCurrentThread,

    /// 永久删除当前活动主线程，并在成功后退出。
    /// Permanently delete the current active main thread and exit after it succeeds.
    DeleteCurrentThread,

    /// 将当前会话派生为新线程，并可选指定名称。
    /// Fork the current session into a new thread, optionally assigning it a name.
    ForkCurrentSession {
        name: Option<String>,
    },

    /// 在选定提示之前创建分支，并在新线程输入框中重新打开该提示。
    /// Branch before a selected prompt and reopen it in the new thread's composer.
    ForkSessionForPromptEdit {
        thread_id: ThreadId,
        nth_user_message: usize,
        prompt: UserMessage,
    },

    /// 请求退出应用；用户主动退出应先关闭核心，立即退出仅作为可能丢失在途工作的最后手段。
    /// Request to exit the application.
    ///
    /// Use `ShutdownFirst` for user-initiated quits so core cleanup runs and the
    /// UI exits only after `ShutdownComplete`. `Immediate` is a last-resort
    /// escape hatch that skips shutdown and may drop in-flight work (e.g.,
    /// background tasks, rollout flush, or child process cleanup).
    Exit(ExitMode),

    /// 将运行中任务退出菜单的选择应用到其来源线程。
    /// Apply a choice from the running-task exit menu to its originating thread.
    RunningTaskExit {
        action: RunningTaskExitAction,
        thread_id: ThreadId,
    },

    /// 请求应用服务器账户登出，并在成功后退出。
    /// Request app-server account logout, then exit after it succeeds.
    Logout,

    /// 因致命错误请求退出应用。
    /// Request to exit the application due to a fatal error.
    #[allow(dead_code)]
    FatalExitRequest(String),

    /// 向代理转发命令；通过 `AppEvent` 可避免在多层组件间逐级传递通道。
    /// Forward a command to the Agent. Using an `AppEvent` for this avoids
    /// bubbling channels through layers of widgets.
    CodexOp(AppCommand),

    /// 批准重试 TUI 中选定的一条近期自动审查拒绝。
    /// Approve one retry of a recent auto-review denial selected in the TUI.
    ApproveRecentAutoReviewDenial {
        thread_id: ThreadId,
        id: String,
    },

    /// 为给定查询（`@` 后文本）启动异步文件搜索；应用层可取消旧搜索，保证最多一个在途请求。
    /// Kick off an asynchronous file search for the given query (text after
    /// the `@`). Previous searches may be cancelled by the app layer so there
    /// is at most one in-flight search.
    StartFileSearch(String),

    /// 异步文件搜索结果；回显 `query` 使界面能判断结果是否仍与当前输入相关。
    /// Result of a completed asynchronous file search. The `query` echoes the
    /// original search term so the UI can decide whether the results are
    /// still relevant.
    FileSearchResult {
        query: String,
        matches: Vec<FileMatch>,
    },

    /// 在后台刷新账户限额。
    /// Refresh account rate limits in the background.
    RefreshRateLimits {
        origin: RateLimitRefreshOrigin,
    },

    /// 打开当前线程的目标摘要及动作菜单。
    /// Open the current thread goal summary/action menu.
    OpenThreadGoalMenu {
        thread_id: ThreadId,
    },

    /// 为当前线程目标内容打开编辑器。
    /// Open an editor for the current thread goal objective.
    OpenThreadGoalEditor {
        thread_id: Option<ThreadId>,
    },

    /// 物化并设置或替换当前线程目标内容。
    /// Materialize and set or replace the current thread goal objective.
    SetThreadGoalDraft {
        thread_id: ThreadId,
        draft: GoalDraft,
        mode: ThreadGoalSetMode,
    },

    /// 暂停或恢复当前线程目标。
    /// Pause or resume the current thread goal.
    SetThreadGoalStatus {
        thread_id: ThreadId,
        status: ThreadGoalStatus,
    },

    /// 清除当前线程目标。
    /// Clear the current thread goal.
    ClearThreadGoal {
        thread_id: ThreadId,
    },

    /// 刷新限额的结果。
    /// Result of refreshing rate limits.
    RateLimitsLoaded {
        origin: RateLimitRefreshOrigin,
        hard_stop_generation: u64,
        result: Result<GetAccountRateLimitsResponse, String>,
    },

    /// 打开 `/usage` 菜单选定的默认令牌活动视图。
    /// Open the default token-activity view selected from the `/usage` menu.
    OpenTokenActivity,

    /// 打开 `/usage` 菜单选定的重置额度流程。
    /// Open the reset-credit flow selected from the `/usage` menu.
    OpenRateLimitResetCredits,

    /// 确认重置额度选择器中选定的额度。
    /// Confirm the reset credit selected from the reset-credit picker.
    OpenRateLimitResetConfirmation {
        picker_request_id: u64,
        confirmation_gate: Arc<AtomicBool>,
        credit_id: Option<String>,
        reset_title: String,
        reset_detail: Option<String>,
        reset_description: String,
    },

    /// 使用稳定幂等键消费一个重置额度。
    /// Consume one reset credit using a stable idempotency key.
    ConsumeRateLimitResetCredit {
        idempotency_key: String,
        credit_id: Option<String>,
    },

    /// 消费重置额度的结果。
    /// Result of consuming one reset credit.
    RateLimitResetCreditConsumed {
        request_id: u64,
        idempotency_key: String,
        credit_id: Option<String>,
        result: Result<ConsumeAccountRateLimitResetCreditResponse, String>,
    },

    /// 为 `/usage` 历史卡片获取账户级令牌活动。
    /// Fetch account-wide token activity for a `/usage` history card.
    RefreshTokenActivity {
        request_id: u64,
    },

    /// 获取账户级令牌活动的结果。
    /// Result of fetching account-wide token activity.
    TokenActivityLoaded {
        request_id: u64,
        result: Result<GetAccountTokenUsageResponse, String>,
    },

    /// 获取当前可见企业线程的后端估算用量。
    /// Fetch backend-estimated usage for the currently visible enterprise thread.
    RefreshThreadUsage {
        thread_id: ThreadId,
        request_id: u64,
    },

    /// 获取指定线程后端估算用量的结果。
    /// Result of fetching backend-estimated usage for a specific thread.
    ThreadUsageLoaded {
        thread_id: ThreadId,
        request_id: u64,
        result: Result<ThreadUsageOutcome, String>,
    },

    /// 为状态行标题项获取工作区消息。
    /// Fetch workspace messages for the status-line headline item.
    RefreshStatusLineWorkspaceHeadline {
        request_id: u64,
    },

    /// 活动输出屏障清除后提交已稳定的异步用量输出。
    /// Commit settled asynchronous usage output after active-output barriers clear.
    CommitPendingUsageOutput,

    /// 流关闭后提交已稳定的异步用量输出。
    /// Commit settled asynchronous usage output after stream shutdown.
    CommitPendingUsageOutputAfterStreamShutdown,

    /// 发送经用户确认的工作区所有者通知请求。
    /// Send a user-confirmed request to notify the workspace owner.
    SendAddCreditsNudgeEmail {
        credit_type: AddCreditsNudgeCreditType,
    },

    /// 通知工作区所有者的结果。
    /// Result of notifying the workspace owner.
    AddCreditsNudgeEmailFinished {
        result: Result<AddCreditsNudgeEmailStatus, String>,
    },

    /// 预取连接器的结果。
    /// Result of prefetching connectors.
    ConnectorsLoaded {
        thread_id: Option<ThreadId>,
        cwd: PathBuf,
        generation: ConnectorScopeGeneration,
        result: Result<ConnectorsSnapshot, String>,
        is_final: bool,
    },

    /// 当前线程范围内可实际提及的已安装应用。
    /// Thread-scoped installed applications that may actually be mentioned.
    InstalledConnectorMentionsLoaded {
        thread_id: Option<ThreadId>,
        cwd: PathBuf,
        generation: ConnectorScopeGeneration,
        result: Result<ConnectorsSnapshot, String>,
    },

    /// 计算 `/diff` 命令的结果。
    /// Result of computing a `/diff` command.
    DiffResult(PathBuf, String),

    /// 在底部面板打开应用链接视图。
    /// Open the app link view in the bottom pane.
    OpenAppLink {
        app_id: String,
        title: String,
        description: Option<String>,
        instructions: String,
        url: String,
        is_installed: bool,
        is_enabled: bool,
    },

    /// 在用户浏览器中打开给定 URL。
    /// Open the provided URL in the user's browser.
    OpenUrlInBrowser {
        url: String,
    },

    /// 在 Codex Desktop 中打开当前线程。
    /// Open the current thread in Codex Desktop.
    OpenDesktopThread {
        thread_id: ThreadId,
    },

    /// 持久化宠物选择并重新加载环境宠物。
    /// Persist a pet selection and reload the ambient pet.
    PetSelected {
        pet_id: String,
    },

    /// 持久化终端宠物禁用状态并移除环境宠物。
    /// Persist terminal pets as disabled and remove the ambient pet.
    PetDisabled,

    /// 开始加载宠物选择器的侧边预览。
    /// Start loading the side preview for the pet picker.
    PetPreviewRequested {
        pet_id: String,
    },

    /// 加载宠物选择器侧边预览的结果。
    /// Result of loading the side preview for the pet picker.
    PetPreviewLoaded {
        request_id: u64,
        result: Result<crate::pets::AmbientPet, String>,
    },

    /// 配置持久化前加载所选环境宠物的结果。
    /// Result of loading the selected ambient pet before config persistence.
    PetSelectionLoaded {
        request_id: u64,
        pet_id: String,
        result: Result<Option<crate::pets::AmbientPet>, String>,
    },

    /// 启动期间恢复已配置环境宠物的结果。
    /// Result of restoring the configured ambient pet during startup.
    ConfiguredPetLoaded {
        pet_id: String,
        result: Result<Option<crate::pets::AmbientPet>, String>,
    },

    /// 刷新应用连接器状态和提及绑定。
    /// Refresh app connector state and mention bindings.
    RefreshConnectors {
        force_refetch: bool,
    },

    /// 仅在来源账户、工作区和线程仍为当前值时获取应用。
    /// Fetch apps only while the originating account, workspace, and thread remain current.
    FetchConnectorsList {
        force_refetch: bool,
        generation: ConnectorScopeGeneration,
    },

    /// 刷新可调用的已安装应用，但不加载应用目录。
    /// Refresh callable installed applications without loading the app directory.
    FetchInstalledConnectorMentions {
        force_refresh: bool,
        generation: ConnectorScopeGeneration,
    },

    /// 获取指定工作目录的插件市场状态。
    /// Fetch plugin marketplace state for the provided working directory.
    FetchPluginsList {
        cwd: PathBuf,
    },

    /// 获取指定工作目录的生命周期钩子清单。
    /// Fetch lifecycle hook inventory for the provided working directory.
    FetchHooksList {
        cwd: PathBuf,
    },

    /// 获取插件市场状态的结果。
    /// Result of fetching plugin marketplace state.
    PluginsLoaded {
        cwd: PathBuf,
        result: Result<PluginListResponse, String>,
    },

    /// 从已缓存响应打开插件列表。
    /// Open the plugin list from an already cached response.
    OpenPluginsList {
        cwd: PathBuf,
        response: PluginListResponse,
    },

    /// 显式获取远端支持插件分区的结果。
    /// Result of explicitly fetching remote-backed plugin sections.
    PluginRemoteSectionsLoaded {
        cwd: PathBuf,
        marketplaces: Vec<PluginMarketplaceEntry>,
        section_errors: Vec<PluginRemoteSectionError>,
    },

    /// 获取生命周期钩子清单的结果。
    /// Result of fetching lifecycle hook inventory.
    HooksLoaded {
        cwd: PathBuf,
        result: Result<codex_app_server_protocol::HooksListResponse, String>,
    },

    /// 打开添加市场来源的提示。
    /// Open the prompt for adding a marketplace source.
    OpenMarketplaceAddPrompt,

    /// 将插件弹窗替换为添加市场的加载状态。
    /// Replace the plugins popup with a marketplace-add loading state.
    OpenMarketplaceAddLoading {
        source: String,
    },

    /// 从给定来源添加市场。
    /// Add a marketplace from the provided source.
    FetchMarketplaceAdd {
        cwd: PathBuf,
        source: String,
    },

    /// 添加市场的结果。
    /// Result of adding a marketplace.
    MarketplaceAddLoaded {
        cwd: PathBuf,
        source: String,
        result: Result<MarketplaceAddResponse, String>,
    },

    /// 打开移除市场的确认提示。
    /// Open the confirmation prompt for removing a marketplace.
    OpenMarketplaceRemoveConfirm {
        marketplace_name: String,
        marketplace_display_name: String,
    },

    /// 将插件弹窗替换为移除市场的加载状态。
    /// Replace the plugins popup with a marketplace-remove loading state.
    OpenMarketplaceRemoveLoading {
        marketplace_display_name: String,
    },

    /// 按名称移除市场。
    /// Remove a marketplace by name.
    FetchMarketplaceRemove {
        cwd: PathBuf,
        marketplace_name: String,
        marketplace_display_name: String,
    },

    /// 移除市场的结果。
    /// Result of removing a marketplace.
    MarketplaceRemoveLoaded {
        cwd: PathBuf,
        marketplace_name: String,
        marketplace_display_name: String,
        result: Result<MarketplaceRemoveResponse, String>,
    },

    /// 将插件弹窗替换为升级市场的加载状态。
    /// Replace the plugins popup with a marketplace-upgrade loading state.
    OpenMarketplaceUpgradeLoading {
        marketplace_name: Option<String>,
    },

    /// 升级已配置的 Git 市场。
    /// Upgrade configured Git marketplaces.
    FetchMarketplaceUpgrade {
        cwd: PathBuf,
        marketplace_name: Option<String>,
    },

    /// 升级已配置 Git 市场的结果。
    /// Result of upgrading configured Git marketplaces.
    MarketplaceUpgradeLoaded {
        cwd: PathBuf,
        result: Result<MarketplaceUpgradeResponse, String>,
    },

    /// 将插件弹窗替换为插件详情加载状态。
    /// Replace the plugins popup with a plugin-detail loading state.
    OpenPluginDetailLoading {
        plugin_display_name: String,
    },

    /// 从市场获取指定插件详情。
    /// Fetch detail for a specific plugin from a marketplace.
    FetchPluginDetail {
        cwd: PathBuf,
        params: PluginReadParams,
    },

    /// 获取插件详情的结果。
    /// Result of fetching plugin detail.
    PluginDetailLoaded {
        cwd: PathBuf,
        result: Result<PluginReadResponse, String>,
    },

    /// 将插件弹窗替换为安装加载状态。
    /// Replace the plugins popup with an install loading state.
    OpenPluginInstallLoading {
        plugin_display_name: String,
    },

    /// 将插件弹窗替换为卸载加载状态。
    /// Replace the plugins popup with an uninstall loading state.
    OpenPluginUninstallLoading {
        plugin_display_name: String,
    },

    /// 从市场安装指定插件。
    /// Install a specific plugin from a marketplace.
    FetchPluginInstall {
        cwd: PathBuf,
        location: PluginLocation,
        plugin_name: String,
        plugin_display_name: String,
    },

    /// 安装插件的结果。
    /// Result of installing a plugin.
    PluginInstallLoaded {
        cwd: PathBuf,
        location: PluginLocation,
        plugin_name: String,
        plugin_display_name: String,
        result: Result<PluginInstallResponse, String>,
    },

    /// 按规范插件 ID 卸载指定插件。
    /// Uninstall a specific plugin by canonical plugin id.
    FetchPluginUninstall {
        cwd: PathBuf,
        plugin_id: String,
        plugin_display_name: String,
    },

    /// 卸载插件的结果。
    /// Result of uninstalling a plugin.
    PluginUninstallLoaded {
        cwd: PathBuf,
        plugin_id: String,
        plugin_display_name: String,
        result: Result<PluginUninstallResponse, String>,
    },

    /// 启用或禁用已安装插件。
    /// Enable or disable an installed plugin.
    SetPluginEnabled {
        cwd: PathBuf,
        plugin_id: String,
        enabled: bool,
    },

    /// 启用或禁用插件的结果。
    /// Result of enabling or disabling a plugin.
    PluginEnabledSet {
        cwd: PathBuf,
        plugin_id: String,
        enabled: bool,
        result: Result<(), String>,
    },

    /// 从当前配置刷新插件提及绑定。
    /// Refresh plugin mention bindings from the current config.
    RefreshPluginMentions,

    /// 刷新插件提及绑定的结果。
    /// Result of refreshing plugin mention bindings.
    PluginMentionsLoaded {
        cwd: PathBuf,
        plugins: Option<Vec<PluginCapabilitySummary>>,
    },

    /// 推进插件安装后的应用授权流程。
    /// Advance the post-install plugin app-auth flow.
    PluginInstallAuthAdvance {
        refresh_connectors: bool,
    },

    /// 放弃插件安装后的应用授权流程。
    /// Abandon the post-install plugin app-auth flow.
    PluginInstallAuthAbandon,

    /// 通过应用服务器 RPC 获取 MCP 清单并渲染到历史。
    /// Fetch MCP inventory via app-server RPCs and render it into history.
    FetchMcpInventory {
        detail: McpServerStatusDetail,
        thread_id: Option<ThreadId>,
    },

    /// 通过应用服务器 RPC 获取 MCP 清单的结果。
    /// Result of fetching MCP inventory via app-server RPCs.
    McpInventoryLoaded {
        result: Result<Vec<McpServerStatus>, String>,
        detail: McpServerStatusDetail,
        thread_id: Option<ThreadId>,
    },

    /// 首帧安排后执行的启动技能刷新结果；该事件仅用于启动，交互刷新同步走应用命令路径。
    /// Result of the startup skills refresh that runs after the first frame is scheduled.
    ///
    /// This event is startup-only. Interactive skills refreshes are handled synchronously through the app
    /// command path because those callers expect the visible skill state to be current when their command
    /// completes.
    SkillsListLoaded {
        cwd: PathBuf,
        result: Result<SkillsListResponse, String>,
    },

    /// 在初始恢复重放行写入回滚区前开始缓冲。
    /// Begin buffering initial resume replay rows before they are written to scrollback.
    BeginInitialHistoryReplayBuffer,

    /// 开始缓冲线程切换重放单元，使最终写回可复用尺寸重排尾部渲染器。
    /// Begin buffering thread-switch replay cells so the final scrollback write can reuse the
    /// resize-reflow tail renderer.
    BeginThreadSwitchHistoryReplayBuffer,

    InsertHistoryCell(Box<dyn HistoryCell>),

    /// 所有重放事件入队后结束初始恢复缓冲。
    /// Finish buffering initial resume replay after all replay events have been queued.
    EndInitialHistoryReplayBuffer,

    /// 将记录末尾连续的流式 `AgentMessageCell` 替换为保存原始 Markdown、可随尺寸重绘的
    /// 单个 `AgentMarkdownCell`；同时保留 cwd、回滚区强制重排和延迟尾单元语义。
    /// Replace the contiguous run of streaming `AgentMessageCell`s at the end of
    /// the transcript with a single `AgentMarkdownCell` that stores the raw
    /// markdown source and re-renders from it on resize.
    ///
    /// Emitted by `ChatWidget::flush_answer_stream_with_separator` after stream
    /// finalization. The `App` handler walks backward through `transcript_cells`
    /// to find the `AgentMessageCell` run and splices in the consolidated cell.
    /// The `cwd` keeps local file-link display stable across the final re-render.
    /// `scrollback_reflow` lets table-tail finalization force the already-emitted
    /// terminal scrollback to be rebuilt from the consolidated source-backed cell.
    /// `deferred_history_cell` lets callers add the final stream tail to the
    /// transcript without first writing its provisional render to scrollback.
    ConsolidateAgentMessage {
        source: String,
        cwd: PathBuf,
        inline_visualization_context: Option<InlineVisualizationContext>,
        scrollback_reflow: ConsolidationScrollbackReflow,
        deferred_history_cell: Option<Box<dyn HistoryCell>>,
    },

    /// 计划流结束后，将末尾连续 `ProposedPlanStreamCell` 替换为来源支持的单个 `ProposedPlanCell`。
    /// Replace the contiguous run of streaming `ProposedPlanStreamCell`s at the
    /// end of the transcript with a single source-backed `ProposedPlanCell`.
    ///
    /// Emitted by `ChatWidget::on_plan_item_completed` after plan stream
    /// finalization.
    ConsolidateProposedPlan(String),

    StartCommitAnimation,
    StopCommitAnimation,
    CommitTick,

    /// 更新运行中应用和组件的当前推理强度。
    /// Update the current reasoning effort in the running app and widget.
    UpdateReasoningEffort(Option<ReasoningEffort>),

    /// 更新运行中应用和组件的当前模型标识。
    /// Update the current model slug in the running app and widget.
    UpdateModel(String),

    /// 更新运行中应用和组件的当前个性。
    /// Update the current personality in the running app and widget.
    UpdatePersonality(Personality),

    /// 前置更新事件应用后结束设置选择。
    /// Finish a settings selection after its preceding update events have been applied.
    SettingsSelectionClosed,
    /// 处理关闭事件期间发出的嵌套设置事件完成后运行。
    /// Run after any nested settings events emitted while handling the close event.
    SettingsSelectionSettled,

    /// 将选定模型和推理强度持久化到相应配置。
    /// Persist the selected model and reasoning effort to the appropriate config.
    PersistModelSelection {
        model: String,
        effort: Option<ReasoningEffort>,
    },

    /// 模型选择确认后展示 cyber 自动审查提示。
    /// Show the cyber auto-review notice after the model selection confirmation.
    CyberModelAutoReviewNotice,

    /// 将选定个性持久化到相应配置。
    /// Persist the selected personality to the appropriate config.
    PersistPersonalitySelection {
        personality: Personality,
    },

    /// 将选定服务层级持久化到相应配置。
    /// Persist the selected service tier to the appropriate config.
    PersistServiceTierSelection {
        service_tier: Option<String>,
    },

    /// 选择模型后打开推理强度弹窗。
    /// Open the reasoning selection popup after picking a model.
    OpenReasoningPopup {
        model: ModelPreset,
    },

    /// 为模型打开显式 Max/Ultra 推理选择弹窗。
    /// Open the explicit Max/Ultra reasoning selection popup for a model.
    OpenAdvancedReasoningPopup {
        model: ModelPreset,
    },

    /// 将高级推理强度应用到活动会话，但不修改默认值。
    /// Apply an advanced reasoning effort to the active conversation without changing defaults.
    ApplyAdvancedReasoning {
        model: String,
        effort: ReasoningEffort,
    },

    /// 为选定模型和强度打开计划模式推理范围提示。
    /// Open the Plan-mode reasoning scope prompt for the selected model/effort.
    OpenPlanReasoningScopePrompt {
        model: String,
        effort: Option<ReasoningEffort>,
    },

    /// 打开非自动模型的完整选择器。
    /// Open the full model picker (non-auto models).
    OpenAllModelsPopup {
        models: Vec<ModelPreset>,
    },

    /// 启用完全访问模式前打开确认提示。
    /// Open the confirmation prompt before enabling full access mode.
    OpenFullAccessConfirmation {
        preset: ApprovalPreset,
        return_to_permissions: bool,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 仅在来源线程仍显示时应用权限快捷选择。
    /// Apply a permission shortcut only while its originating thread is displayed.
    ApplyPermissionShortcut {
        thread_id: ThreadId,
        selection: PermissionProfileSelection,
    },

    /// 打开 Windows 全局可写目录警告；有预设时继续操作会应用审批/沙箱配置，
    /// 无预设时只确认或关闭警告，不改变策略。
    /// Open the Windows world-writable directories warning.
    /// If `preset` is `Some`, the confirmation will apply the provided
    /// approval/sandbox configuration on Continue; if `None`, it performs no
    /// policy change and only acknowledges/dismisses the warning.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    OpenWorldWritableWarningConfirmation {
        preset: Option<ApprovalPreset>,
        profile_selection: Option<PermissionProfileSelection>,
        /// 警告中最多展示三个全局可写目录样例。
        /// Up to 3 sample world-writable directories to display in the warning.
        sample_paths: Vec<String>,
        /// 若实际目录多于 `sample_paths`，此字段记录剩余数量。
        /// If there are more than `sample_paths`, this carries the remaining count.
        extra_count: usize,
        /// 扫描失败（如 ACL 查询错误）且无法验证保护措施时为真。
        /// True when the scan failed (e.g. ACL query error) and protections could not be verified.
        failed_scan: bool,
    },

    /// 使用代理模式前提示启用 Windows 沙箱功能。
    /// Prompt to enable the Windows sandbox feature before using Agent mode.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    OpenWindowsSandboxEnablePrompt {
        preset: ApprovalPreset,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 拒绝提权或提权失败后打开 Windows 沙箱回退提示。
    /// Open the Windows sandbox fallback prompt after declining or failing elevation.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    OpenWindowsSandboxFallbackPrompt {
        preset: ApprovalPreset,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 开始需提权的 Windows 沙箱设置流程。
    /// Begin the elevated Windows sandbox setup flow.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    BeginWindowsSandboxElevatedSetup {
        preset: ApprovalPreset,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 开始无需提权的 Windows 沙箱设置流程。
    /// Begin the non-elevated Windows sandbox setup flow.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    BeginWindowsSandboxLegacySetup {
        preset: ApprovalPreset,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 开始无需提权地授予额外目录读取权限。
    /// Begin a non-elevated grant of read access for an additional directory.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    BeginWindowsSandboxGrantReadRoot {
        path: String,
    },

    /// 尝试授予额外目录读取权限的结果。
    /// Result of attempting to grant read access for an additional directory.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    WindowsSandboxGrantReadRootCompleted {
        path: PathBuf,
        error: Option<String>,
    },

    /// 启用 Windows 沙箱功能并切换到代理模式。
    /// Enable the Windows sandbox feature and switch to Agent mode.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    EnableWindowsSandboxForAgentMode {
        preset: ApprovalPreset,
        mode: WindowsSandboxEnableMode,
        profile_selection: Option<PermissionProfileSelection>,
    },

    /// 更新 Windows 沙箱功能模式，但不修改审批预设。
    /// Update the Windows sandbox feature mode without changing approval presets.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]

    /// 更新运行中应用和组件的当前审批策略。
    /// Update the current approval policy in the running app and widget.
    UpdateAskForApprovalPolicy(AskForApproval),

    /// 更新运行中应用和组件的当前内置活动权限配置。
    /// Update the current built-in active permission profile in the running app and widget.
    UpdateActivePermissionProfile(ActivePermissionProfile),

    /// 选择命名权限配置，并可选同时应用内置模式设置。
    /// Select a named permission profile, optionally applying built-in mode settings too.
    SelectPermissionProfile(PermissionProfileSelection),

    /// 更新运行中应用和组件的当前审批复核者。
    /// Update the current approvals reviewer in the running app and widget.
    UpdateApprovalsReviewer(ApprovalsReviewer),

    /// 更新功能开关并持久化到顶层配置。
    /// Update feature flags and persist them to the top-level config.
    UpdateFeatureFlags {
        updates: Vec<(Feature, bool)>,
    },

    /// 更新记忆设置并持久化到 `config.toml`。
    /// Update memory settings and persist them to config.toml.
    UpdateMemorySettings {
        use_memories: bool,
        generate_memories: bool,
    },

    /// 通过应用服务器清除全部持久化本地记忆工件。
    /// Clear all persisted local memory artifacts via the app-server.
    ResetMemories,

    /// 更新是否已确认全局可写目录警告。
    /// Update whether the world-writable directories warning has been acknowledged.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    UpdateWorldWritableWarningAcknowledged(bool),

    /// 更新本会话是否已确认限额切换提示。
    /// Update whether the rate limit switch prompt has been acknowledged for the session.
    UpdateRateLimitSwitchPromptHidden(bool),

    /// 在内存中更新计划模式专用推理强度。
    /// Update the Plan-mode-specific reasoning effort in memory.
    UpdatePlanModeReasoningEffort(Option<ReasoningEffort>),

    /// 持久化全局可写目录警告的确认标记。
    /// Persist the acknowledgement flag for the world-writable directories warning.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    PersistWorldWritableWarningAcknowledged,

    /// 持久化限额切换提示的确认标记。
    /// Persist the acknowledgement flag for the rate limit switch prompt.
    PersistRateLimitSwitchPromptHidden,

    /// 持久化计划模式专用推理强度。
    /// Persist the Plan-mode-specific reasoning effort.
    PersistPlanModeReasoningEffort(Option<ReasoningEffort>),

    /// 持久化模型迁移提示的确认标记。
    /// Persist the acknowledgement flag for the model migration prompt.
    PersistModelMigrationPromptAcknowledged {
        from_model: String,
        to_model: String,
    },

    /// 用户确认继续后一次性跳过下一次全局可写扫描。
    /// Skip the next world-writable scan (one-shot) after a user-confirmed continue.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    SkipNextWorldWritableScan,

    /// 重新打开审批预设弹窗。
    /// Re-open the approval presets popup.
    OpenApprovalsPopup,

    /// 打开技能列表弹窗。
    /// Open the skills list popup.
    OpenSkillsList,

    /// 打开技能启用/禁用选择器。
    /// Open the skills enable/disable picker.
    OpenManageSkillsPopup,

    /// 按路径启用或禁用技能。
    /// Enable or disable a skill by path.
    SetSkillEnabled {
        path: AbsolutePathBuf,
        enabled: bool,
    },

    /// 按连接器 ID 启用或禁用应用。
    /// Enable or disable an app by connector ID.
    SetAppEnabled {
        id: String,
        enabled: bool,
    },

    /// 按稳定钩子键启用或禁用钩子。
    /// Enable or disable a hook by stable hook key.
    SetHookEnabled {
        key: String,
        enabled: bool,
    },

    /// 按稳定钩子键信任该钩子的当前定义。
    /// Trust the current definition for a hook by stable hook key.
    TrustHook {
        key: String,
        current_hash: String,
    },

    /// 按稳定钩子键信任一个或多个钩子的当前定义。
    /// Trust the current definitions for one or more hooks by stable hook key.
    TrustHooks {
        updates: Vec<crate::hooks_rpc::HookTrustUpdate>,
    },

    /// 持久化钩子启用状态的结果。
    /// Result of persisting hook enabled state.
    HookEnabledSet {
        key: String,
        enabled: bool,
        result: Result<(), String>,
    },

    /// 持久化钩子信任状态的结果。
    /// Result of persisting hook trust state.
    HookTrusted {
        result: Result<(), String>,
    },

    /// 通知技能管理弹窗已关闭。
    /// Notify that the manage skills popup was closed.
    ManageSkillsClosed,

    /// 重新打开权限预设弹窗。
    /// Re-open the permissions presets popup.
    OpenPermissionsPopup,

    /// 从审查弹窗打开分支选择器选项。
    /// Open the branch picker option from the review popup.
    OpenReviewBranchPicker(PathBuf),

    /// 从审查弹窗打开提交选择器选项。
    /// Open the commit picker option from the review popup.
    OpenReviewCommitPicker(PathBuf),

    /// 从审查弹窗打开自定义提示选项。
    /// Open the custom prompt option from the review popup.
    OpenReviewCustomPrompt,

    /// 使用显式协作模式掩码提交用户消息。
    /// Submit a user message with an explicit collaboration mask.
    SubmitUserMessageWithMode {
        text: String,
        collaboration_mode: CollaborationModeMask,
    },

    /// 打开审批弹窗。
    /// Open the approval popup.
    FullScreenApprovalRequest(ApprovalRequest),

    /// 用户选择分类后打开反馈备注输入覆盖层。
    /// Open the feedback note entry overlay after the user selects a category.
    OpenFeedbackNote {
        category: FeedbackCategory,
        include_logs: bool,
    },

    /// 选择分类后打开发送反馈的上传同意弹窗。
    /// Open the upload consent popup for feedback after selecting a category.
    OpenFeedbackConsent {
        category: FeedbackCategory,
    },

    /// 通过应用服务器反馈 RPC 提交当前线程反馈。
    /// Submit feedback for the current thread via the app-server feedback RPC.
    SubmitFeedback {
        category: FeedbackCategory,
        reason: Option<String>,
        turn_id: Option<String>,
        include_logs: bool,
    },

    /// TUI 发起的反馈上传请求结果。
    /// Result of a feedback upload request initiated by the TUI.
    FeedbackSubmitted {
        origin_thread_id: Option<ThreadId>,
        category: FeedbackCategory,
        include_logs: bool,
        result: Result<String, String>,
    },

    /// 正常绘制完成后启动外部编辑器。
    /// Launch the external editor after a normal draw has completed.
    LaunchExternalEditor,

    /// 用于状态行渲染的当前 Git 分支异步更新。
    /// Async update of the current git branch for status line rendering.
    StatusLineBranchUpdated {
        cwd: PathBuf,
        branch: Option<String>,
    },
    /// 用于状态行渲染的 Git 摘要字段异步更新。
    /// Async update of Git summary fields for status line rendering.
    StatusLineGitSummaryUpdated {
        cwd: PathBuf,
        summary: crate::chatwidget::StatusLineGitSummary,
    },
    /// 用于状态行渲染的工作区通知标题异步更新。
    /// Async update of the workspace notification headline for status line rendering.
    StatusLineWorkspaceHeadlineUpdated {
        request_id: u64,
        result: Result<crate::workspace_messages::WorkspaceHeadlineFetchResult, String>,
    },
    /// 应用用户确认的状态行项目顺序和选择。
    /// Apply a user-confirmed status-line item ordering/selection.
    StatusLineSetup {
        items: Vec<StatusLineItem>,
        use_theme_colors: bool,
    },
    /// 关闭状态行设置界面但不修改配置。
    /// Dismiss the status-line setup UI without changing config.
    StatusLineSetupCancelled,

    /// 应用用户确认的终端标题项目顺序和选择。
    /// Apply a user-confirmed terminal-title item ordering/selection.
    TerminalTitleSetup {
        items: Vec<TerminalTitleItem>,
    },
    /// 设置界面打开期间应用临时终端标题预览。
    /// Apply a temporary terminal-title preview while the setup UI is open.
    TerminalTitleSetupPreview {
        items: Vec<TerminalTitleItem>,
    },
    /// 关闭终端标题设置界面但不修改配置。
    /// Dismiss the terminal-title setup UI without changing config.
    TerminalTitleSetupCancelled,

    /// 应用用户确认的语法主题选择。
    /// Apply a user-confirmed syntax theme selection.
    SyntaxThemeSelected {
        name: String,
    },

    /// 运行时语法主题预览改变后刷新主题派生的界面颜色。
    /// Runtime syntax theme preview changed; refresh theme-derived UI colors.
    SyntaxThemePreviewed,

    /// 为选定键位动作打开设置/移除操作。
    /// Open set/remove actions for the selected keymap action.
    OpenKeymapActionMenu {
        context: String,
        action: String,
    },

    /// 替换动作的某个绑定前打开绑定选择。
    /// Open binding selection before replacing one binding for an action.
    OpenKeymapReplaceBindingMenu {
        context: String,
        action: String,
    },

    /// 为选定键位动作打开按键捕获。
    /// Open key capture for the selected keymap action.
    OpenKeymapCapture {
        context: String,
        action: String,
        intent: KeymapEditIntent,
        capture_mode: KeymapCaptureMode,
    },

    /// 打开键位按键检查器。
    /// Open the keymap keypress inspector.
    OpenKeymapDebug,

    /// 把捕获的按键应用到选定键位动作。
    /// Apply a captured key to the selected keymap action.
    KeymapCaptured {
        context: String,
        action: String,
        key: String,
        intent: KeymapEditIntent,
    },

    /// 移除选定键位动作的自定义根绑定。
    /// Remove the custom root binding for the selected keymap action.
    KeymapCleared {
        context: String,
        action: String,
    },
}

/// 所需界面防护完成后要应用的命名权限配置选择。
/// Named profile selection to apply after any required UI guardrails complete.
#[derive(Debug, Clone)]
pub(crate) struct PermissionProfileSelection {
    pub profile_id: String,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub display_label: String,
}

/// 界面层请求的退出策略；用户退出通常先关闭核心，`Immediate` 只用于已关闭或明确绕过的场景。
/// The exit strategy requested by the UI layer.
///
/// Most user-initiated exits should use `ShutdownFirst` so core cleanup runs and the UI exits only
/// after core acknowledges completion. `Immediate` is an escape hatch for cases where shutdown has
/// already completed (or is being bypassed) and the UI loop should terminate right away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitMode {
    /// 关闭核心并在完成后退出。
    /// Shutdown core and exit after completion.
    ShutdownFirst,
    /// 不等待关闭而立即退出界面循环；这会跳过 `Op::Shutdown`，可能丢失在途工作和清理。
    /// Exit the UI loop immediately without waiting for shutdown.
    ///
    /// This skips `Op::Shutdown`, so any in-flight work may be dropped and
    /// cleanup that normally runs before `ShutdownComplete` can be missed.
    Immediate,
}

/// 离开仍在运行的守护进程支持任务时所作的选择。
/// Choice made when leaving a daemon-backed task that is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunningTaskExitAction {
    CancelTask,
    RunInBackground,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeedbackCategory {
    BadResult,
    GoodResult,
    Bug,
    SafetyCheck,
    Other,
}
