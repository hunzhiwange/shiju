//! Exhaustive routing of local chat events, independent of host effects.
use super::events::ChatMessage;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    AcpOptions,
    Input,
    Trajectory,
    Stream,
    Context,
    Session,
}
impl<M, P, V, D, T, G> ChatMessage<M, P, V, D, T, G> {
    pub fn route(&self) -> Route {
        match self {
            ChatMessage::AcpModelInputDraftChanged(_)
            | ChatMessage::AcpSourceRefresh
            | ChatMessage::AcpSourceLoaded { .. }
            | ChatMessage::AcpModelSourceSaved { .. }
            | ChatMessage::AcpOptionsOpen
            | ChatMessage::AcpOptionsClose
            | ChatMessage::AcpOptionSelected(_)
            | ChatMessage::AcpOptionsLoaded { .. } => Route::AcpOptions,
            // 输入相关消息：文本编辑、悬停状态、滚动、文件搜索、任务模式输入等
            ChatMessage::InputChanged(_)
            | ChatMessage::QuickMessageSelected(_)
            | ChatMessage::QuickMessageGroupChanged(_)
            | ChatMessage::RecommendedPromptMenuChanged(_)
            | ChatMessage::ToggleShijuSkillMenu
            | ChatMessage::SelectShijuSkill(_)
            | ChatMessage::InputEditorAction(_)
            | ChatMessage::OpenInputContextMenu { .. }
            | ChatMessage::CloseInputContextMenu
            | ChatMessage::CopyInputSelection
            | ChatMessage::CutInputSelection
            | ChatMessage::PasteIntoInput
            | ChatMessage::ClipboardPasteResolved(_)
            | ChatMessage::SelectAllInput
            | ChatMessage::MessageEditorAction(_, _)
            | ChatMessage::SpecialTextEditorAction(_, _, _)
            | ChatMessage::ToolTextEditorAction(_, _, _, _)
            | ChatMessage::MarkdownBlockEditorAction(_, _, _, _)
            | ChatMessage::ThinkEditorAction(_, _, _)
            | ChatMessage::ToggleUserMessage(_)
            | ChatMessage::ToggleThink(_, _, _)
            | ChatMessage::ThinkHover(_, _)
            | ChatMessage::ThinkHoverLeave
            | ChatMessage::ToggleToolFile(_, _, _)
            | ChatMessage::ToggleTool(_, _)
            | ChatMessage::ToolFileHover(_)
            | ChatMessage::ToolFileHoverLeave
            | ChatMessage::ToolHover(_, _)
            | ChatMessage::ToolHoverLeave
            | ChatMessage::ToggleExploreSummary(_, _)
            | ChatMessage::ToggleTodoPanel
            | ChatMessage::TodoAnimTick
            | ChatMessage::ScrollChanged { .. }
            | ChatMessage::VisibleMessageMeasurements(_)
            | ChatMessage::ScrollToBottom
            | ChatMessage::FollowupScrollToBottom { .. }
            | ChatMessage::LocateChatMessage(_)
            | ChatMessage::LocateChatMessageIndex(_)
            | ChatMessage::ToggleHalfFullscreen
            | ChatMessage::ToggleFullscreen
            | ChatMessage::FullscreenOverlayEntered
            | ChatMessage::FullscreenOverlayExited
            | ChatMessage::OpenMessageContextMenu { .. }
            | ChatMessage::OpenChatMessageContextMenu { .. }
            | ChatMessage::CloseMessageContextMenu
            | ChatMessage::CopyContextMenuText
            | ChatMessage::AppendContextMenuText
            | ChatMessage::SearchContextMenuWithBaidu
            | ChatMessage::SearchContextMenuWithGoogle
            | ChatMessage::SearchContextMenuWithBing
            | ChatMessage::FileSearchInputChanged(_)
            | ChatMessage::FileSearchSelect(_)
            | ChatMessage::FileSearchNavigateUp
            | ChatMessage::FileSearchNavigateDown
            | ChatMessage::FileSearchSelectCurrent
            | ChatMessage::SlashPaletteClosed
            | ChatMessage::SlashPaletteNavigateUp
            | ChatMessage::SlashPaletteNavigateDown
            | ChatMessage::SlashPaletteSelectCurrent
            | ChatMessage::SlashPaletteSelect { .. }
            | ChatMessage::SlashPaletteExpandKind(_)
            | ChatMessage::InputAreaDragDrop
            | ChatMessage::InputAreaDragHoverChanged(_)
            | ChatMessage::ToolFilesFilterChanged(_)
            | ChatMessage::OpenToolDetail(_, _, _)
            | ChatMessage::ToolDetailEditorAction(_)
            | ChatMessage::ToolDetailUndo
            | ChatMessage::ToolDetailRedo
            | ChatMessage::ToolDetailOpenContextMenu { .. }
            | ChatMessage::ToolDetailCloseContextMenu
            | ChatMessage::ToolDetailContextMenuCopy
            | ChatMessage::ToolDetailContextMenuCut
            | ChatMessage::ToolDetailContextMenuPaste
            | ChatMessage::ToolDetailContextMenuDelete
            | ChatMessage::ToolDetailEditorWheelScrolled { .. }
            | ChatMessage::ToolDetailScrollbarChanged { .. }
            | ChatMessage::CloseToolDetail
            | ChatMessage::RemoveFileReference(_)
            | ChatMessage::FileReferenceHoverChanged(_)
            | ChatMessage::TaskModeToggled(_)
            | ChatMessage::WorkflowModeToggled(_)
            | ChatMessage::GoalModeToggled(_)
            | ChatMessage::TaskModePriorityChanged(_)
            | ChatMessage::TaskModeExecutorChanged(_)
            | ChatMessage::TaskModeReasoningEffortChanged(_)
            | ChatMessage::TaskModeModelChanged(_)
            | ChatMessage::TaskModeTaskSplitModeChanged(_)
            | ChatMessage::TaskModeSubtaskChanged { .. }
            | ChatMessage::TaskModeSubtaskEditorAction { .. }
            | ChatMessage::TaskModeAddSubtask
            | ChatMessage::TaskModeRemoveSubtask(_)
            | ChatMessage::TaskModeMoveSubtaskUp(_)
            | ChatMessage::TaskModeMoveSubtaskDown(_)
            | ChatMessage::OpenForkSessionDialog(_)
            | ChatMessage::CloseForkSessionDialog
            | ChatMessage::ToggleResetMenu(_)
            | ChatMessage::CloseResetMenu
            | ChatMessage::SetTodoPanelPlacement(_)
            | ChatMessage::ToggleAgentSessionsPopover
            | ChatMessage::CloseAgentSessionsPopover
            | ChatMessage::StopPlanRun(_)
            | ChatMessage::StopPlanRunFinished { .. } => Route::Input,

            ChatMessage::ChatPanelModeSelected(_)
            | ChatMessage::TrajectorySearchChanged(_)
            | ChatMessage::TrajectoryDurationToggled
            | ChatMessage::TrajectoryTurnsToggled
            | ChatMessage::TrajectoryCallsToggled
            | ChatMessage::TrajectoryTurnToggled(_)
            | ChatMessage::TrajectoryToolStepToggled(_)
            | ChatMessage::TrajectoryRecordSelected(_)
            | ChatMessage::TrajectoryDetailsTabSelected(_)
            | ChatMessage::TrajectoryDetailsClosed
            | ChatMessage::TrajectoryDetailsEditorAction(_, _)
            | ChatMessage::TrajectoryDetailsContextMenuOpened { .. }
            | ChatMessage::TrajectoryDetailsContextMenuClosed
            | ChatMessage::TrajectoryDetailsContextMenuCopy
            | ChatMessage::TrajectoryDetailsJsonToggled(_)
            | ChatMessage::TrajectoryDetailsJsonCopied(_)
            | ChatMessage::TrajectoryRawRetry { .. }
            | ChatMessage::TrajectoryRawLoaded { .. }
            | ChatMessage::TrajectorySnapshotLoaded { .. } => Route::Trajectory,

            // 流式响应相关消息：Agent 增量更新、步骤状态、会话标题、问题应答等
            ChatMessage::AgentStreamDelta(_, _)
            | ChatMessage::AgentToolLifecycle(_, _)
            | ChatMessage::McpAppPanelTick
            | ChatMessage::AgentTodoContinuation(_)
            | ChatMessage::AgentCompactionStatus(_, _)
            | ChatMessage::AgentModelRecovery(_, _)
            | ChatMessage::VisibleTaskSessionLoaded(_)
            | ChatMessage::AgentStreamDeltaFlush(_)
            | ChatMessage::AgentStreamSessionSaveFlush(_)
            | ChatMessage::AgentStreamSessionDeltaSaved { .. }
            | ChatMessage::AgentContextLoaded(_, _)
            | ChatMessage::AgentWorkflowNodeUpdate(_, _)
            | ChatMessage::PlanRunSessionStarted { .. }
            | ChatMessage::PlanRunSessionDelta { .. }
            | ChatMessage::PlanRunSessionWaiting(_)
            | ChatMessage::PlanRunSessionFinished(_)
            | ChatMessage::PlanRunSessionCancelled(_)
            | ChatMessage::AgentStepStart(_, _, _, _, _)
            | ChatMessage::AgentStepFinish(_, _, _, _, _, _, _)
            | ChatMessage::AgentStepCostLoaded(_, _, _, _, _)
            | ChatMessage::AgentPostToolRound(_, _, _)
            | ChatMessage::AgentSessionDiffSummary(_, _, _)
            | ChatMessage::SessionSummaryReloaded(_)
            | ChatMessage::SessionSaveAck
            | ChatMessage::SessionSaveFailed(_)
            | ChatMessage::AgentStreamDone(_, _, _, _)
            | ChatMessage::AgentStreamCancelled(_)
            | ChatMessage::AgentStreamStopFinished(_, _)
            | ChatMessage::AgentStreamError(_, _)
            | ChatMessage::DraftResponseReloaded { .. }
            | ChatMessage::DraftReplayChecked { .. }
            | ChatMessage::PublishedSessionTitleLoaded(_)
            | ChatMessage::QuestionPollTick
            | ChatMessage::PermissionPollTick
            | ChatMessage::LoadInputPanelTodos
            | ChatMessage::TodoPollTick
            | ChatMessage::QuestionListLoaded(_)
            | ChatMessage::PermissionListLoaded(_)
            | ChatMessage::InputPanelTodosLoaded(_, _)
            | ChatMessage::QuestionOptionToggled(_, _)
            | ChatMessage::QuestionCustomChanged(_, _)
            | ChatMessage::QuestionSubmit
            | ChatMessage::QuestionReject
            | ChatMessage::QuestionReplySubmitted(_)
            | ChatMessage::QuestionRejected(_)
            | ChatMessage::PermissionApproveOnce
            | ChatMessage::PermissionApproveAlways
            | ChatMessage::PermissionApproveAllAlways
            | ChatMessage::PermissionReject
            | ChatMessage::PermissionSelectRequest(_)
            | ChatMessage::ToggleFullAccessPermission
            | ChatMessage::SessionFullAccessLoaded { .. }
            | ChatMessage::SessionFullAccessSaved { .. }
            | ChatMessage::PermissionReplySubmitted(_) => Route::Stream,

            // 上下文插入相关消息：文件路径、代码片段、位置选择等
            ChatMessage::InsertPosition(_, _, _)
            | ChatMessage::InsertActiveMatch
            | ChatMessage::InsertSelectionRange
            | ChatMessage::InsertSelectionPositions
            | ChatMessage::InsertSelected
            | ChatMessage::AppendText(_) => Route::Context,

            // 会话管理相关消息：发送、取消、模型选择、队列操作等
            ChatMessage::SendPressed
            | ChatMessage::NewSessionProjectResolved { .. }
            | ChatMessage::SendToSession { .. }
            | ChatMessage::SessionSendHistoryLoaded { .. }
            | ChatMessage::CompactSessionRequested { .. }
            | ChatMessage::CompactSessionConfirmationResolved { .. }
            | ChatMessage::CompactSessionFinished { .. }
            | ChatMessage::GenerateGitCommitMessage { .. }
            | ChatMessage::CancelPressed
            | ChatMessage::AcpModelSourceSelected(_)
            | ChatMessage::AutoModelToggled(_)
            | ChatMessage::ReasoningEffortSelected(_)
            | ChatMessage::AgentInstallationStatusesLoaded(_)
            | ChatMessage::AgentRecommendedPromptSelected(_)
            | ChatMessage::AgentInstallationChecked { .. }
            | ChatMessage::SessionAgentSelected(_)
            | ChatMessage::AcpHistoryModeSelected(_)
            | ChatMessage::AcpHistoryRecentCountChanged(_)
            | ChatMessage::ModelSelected(_)
            | ChatMessage::ModelInputChanged(_)
            | ChatMessage::SessionToolBucketToggled(_)
            | ChatMessage::SessionToolSelectorTabSelected(_)
            | ChatMessage::OpenSessionToolSelectorTab(_)
            | ChatMessage::SessionToolSelectorSearchChanged(_)
            | ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(_)
            | ChatMessage::SessionToolGroupCollapsedToggled(_)
            | ChatMessage::SessionToolGroupToolsToggled(_)
            | ChatMessage::SessionToolSelectorSelectAll
            | ChatMessage::SessionToolSelectorInvert
            | ChatMessage::SessionToolToggled(_)
            | ChatMessage::SessionManualToolSelected(_)
            | ChatMessage::SessionSkillSelected(_)
            | ChatMessage::SessionToolSelectorReset
            | ChatMessage::QueueRemove(_)
            | ChatMessage::QueueEdit(_)
            | ChatMessage::QueueGuide(_)
            | ChatMessage::QueueUp(_)
            | ChatMessage::QueueDown(_)
            | ChatMessage::QueueCopy(_)
            | ChatMessage::SubmitTick
            | ChatMessage::ForkSessionAt { .. }
            | ChatMessage::ResetMessageIdsRecovered { .. }
            | ChatMessage::ForkSessionFinished { .. }
            | ChatMessage::ResetSessionToMessage { .. }
            | ChatMessage::ResetSessionFinished { .. } => Route::Session,
        }
    }
}

impl<M, P, V, D, T, G> ChatMessage<M, P, V, D, T, G> {
    pub fn tracks_stream_thinking(&self) -> bool {
        matches!(
            self,
            ChatMessage::AgentStreamDelta(_, _)
                | ChatMessage::AgentToolLifecycle(_, _)
                | ChatMessage::McpAppPanelTick
                | ChatMessage::AgentTodoContinuation(_)
                | ChatMessage::AgentCompactionStatus(_, _)
                | ChatMessage::AgentModelRecovery(_, _)
                | ChatMessage::VisibleTaskSessionLoaded(_)
                | ChatMessage::AgentStreamDeltaFlush(_)
        )
    }
}
