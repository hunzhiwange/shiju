//! 聊天模块主入口
//!
//! 本模块负责处理所有聊天相关的消息和状态更新，包括：
//! - 用户输入处理（文本输入、编辑器操作、文件引用）
//! - Agent 流式响应（消息增量更新、步骤状态、Token 统计）
//! - 任务模式管理（子任务创建、优先级调整、执行器选择）
//! - 会话控制（消息发送、取消、队列管理）
//! - 交互功能（思考块展开、工具调用、问题应答）
//!
//! # 子模块结构
//!
//! - `input`：处理用户输入相关的消息（文本编辑、文件搜索、任务模式输入）
//! - `stream`：处理 Agent 流式响应相关的消息（增量更新、步骤状态、错误处理）
//! - `context`：处理上下文插入相关的消息（文件路径、代码片段、位置选择）
//! - `session`：处理会话管理相关的消息（发送、取消、队列操作、模型选择）

use crate::app::components::chat_panel::message_measure::{
    ChatMessageMeasurement, collect_visible_chat_message_measurements,
};
use crate::app::{App, Message, TodoPanelPlacement, message, models};
use iced::Task;
use iced::widget::{scrollable, text_editor};
use std::time::Duration;
use vw_shared::message::types as agent_message;

pub mod context;
pub mod input;
mod processing_timing;
pub mod session;
pub mod stream;
mod trajectory;

#[cfg(test)]
#[path = "trajectory_tests.rs"]
mod trajectory_tests;

#[cfg(test)]
#[path = "processing_timing_tests.rs"]
mod processing_timing_tests;

const STREAM_AUTOSCROLL_THROTTLE_MS: u64 = 120;
const AUTOSCROLL_HOLD_MS: u64 = 280;

pub(crate) fn refresh_active_trajectory_raw_task(app: &mut App, session_id: &str) -> Task<Message> {
    if app.chat_panel_mode != crate::app::state::ChatPanelMode::Trajectory {
        return Task::none();
    }
    trajectory::refresh_missing_raw_task(app, session_id, None)
}

fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

pub(super) fn arm_autoscroll_hold(app: &mut App) {
    app.chat_auto_scroll = true;
    app.chat_autoscroll_hold_until_ms = now_ms().saturating_add(AUTOSCROLL_HOLD_MS);
}

pub(super) fn focus_chat_on_turn(app: &mut App, user_index: usize) -> bool {
    if user_index >= app.chat_virtualizer.len() {
        return false;
    }

    app.chat_auto_scroll = false;
    app.chat_virtualizer.focus_turn_on_user(user_index);
    app.sync_chat_virtualizer_compat_fields();
    true
}

pub(super) fn scroll_chat_to_bottom_task(app: &mut App) -> Task<Message> {
    arm_autoscroll_hold(app);
    app.chat_virtualizer.snap_to_tail();
    // Scrolling changes the offset, not message heights. Rebuilding the compatibility
    // height index here turns every streaming tail-follow into a full-history update.
    app.sync_chat_scroll_compat_fields();
    iced::widget::operation::snap_to(
        app.chat_scroll_id.clone(),
        scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
    )
    .map(|_: ()| Message::CopyDone)
}

pub(super) fn scroll_chat_to_turn_top_task(app: &mut App, user_index: usize) -> Task<Message> {
    if !focus_chat_on_turn(app, user_index) {
        return Task::none();
    }

    let relative_y = app.chat_virtualizer.relative_y();

    Task::batch([
        iced::widget::operation::snap_to(
            app.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(relative_y) },
        )
        .map(|_: ()| Message::CopyDone),
        measure_visible_chat_messages_task(),
    ])
}

pub(super) fn throttled_stream_autoscroll_task(app: &mut App) -> Option<Task<Message>> {
    if !app.chat_auto_scroll {
        return None;
    }
    let now = now_ms();
    if now.saturating_sub(app.chat_stream_autoscroll_last_ms) < STREAM_AUTOSCROLL_THROTTLE_MS {
        return None;
    }
    app.chat_stream_autoscroll_last_ms = now;
    Some(scroll_chat_to_bottom_task(app))
}

pub(super) fn scroll_chat_to_bottom_with_followups(app: &mut App) -> Task<Message> {
    arm_autoscroll_hold(app);
    app.chat_virtualizer.snap_to_tail();
    app.sync_chat_virtualizer_compat_fields();
    Task::batch([
        iced::widget::operation::snap_to(
            app.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
        )
        .map(|_: ()| Message::CopyDone),
        message::after(Duration::from_millis(16), Message::Chat(ChatMessage::ScrollToBottom)),
        message::after(Duration::from_millis(48), Message::Chat(ChatMessage::ScrollToBottom)),
        message::after(Duration::from_millis(96), Message::Chat(ChatMessage::ScrollToBottom)),
        message::after(Duration::from_millis(160), Message::Chat(ChatMessage::ScrollToBottom)),
    ])
}

pub(super) fn measure_visible_chat_messages_task() -> Task<Message> {
    iced::advanced::widget::operate(collect_visible_chat_message_measurements())
        .map(|measurements| Message::Chat(ChatMessage::VisibleMessageMeasurements(measurements)))
}

fn think_block_key(msg_idx: usize, think_idx: usize) -> u64 {
    ((msg_idx as u64) << 32) | (think_idx as u64)
}

#[derive(Debug, Clone)]
pub enum ClipboardPastePayload {
    Text(String),
    AttachmentPath(String),
    Empty,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkSessionTarget {
    Local,
    NewWorktree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageIndexedSessionAction {
    Fork(ForkSessionTarget),
    Reset { revert_code: bool },
}

#[derive(Debug, Clone)]
pub struct ForkSessionLoaded {
    pub info: vw_shared::session::info::Info,
    pub target: ForkSessionTarget,
    pub messages: Vec<agent_message::WithParts>,
    pub usage: models::TokenUsage,
}

/// 聊天消息枚举
///
/// 定义了聊天界面中所有可能的消息类型，用于在不同组件间传递状态变化和用户操作。
/// 这些消息会被路由到对应的子模块处理函数进行状态更新和副作用执行。
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// 输入内容变化
    ///
    /// 当用户在输入框中输入、删除或修改文本时触发
    InputChanged(String),

    /// 发送按钮按下
    ///
    /// 用户点击发送按钮或按下回车键时触发，开始处理用户消息
    SendPressed,

    /// 点击输入框上方的快捷消息或动作
    QuickMessageSelected(usize),

    /// 点击切换或关闭输入框上方的快捷消息分组菜单
    QuickMessageGroupChanged(Option<vw_config_types::ui::QuickMessageGroup>),

    /// 切换输入框上方的识矩技能菜单
    ToggleShijuSkillMenu,

    /// 选择或取消当前会话的识矩技能
    SelectShijuSkill(u64),

    /// 请求确认压缩指定会话。
    CompactSessionRequested {
        session_id: String,
        project_path: String,
    },

    /// 用户完成压缩确认。
    CompactSessionConfirmationResolved {
        session_id: String,
        project_path: String,
        confirmed: bool,
    },

    /// Gateway 压缩请求完成。
    CompactSessionFinished {
        session_id: String,
        project_path: String,
        result: Result<bool, String>,
    },

    /// 请求当前会话生成 Git 提交标题与摘要，完成后执行对应提交动作
    GenerateGitCommitMessage {
        project_path: String,
        action: crate::app::state::QuickMessageGitCommitAction,
    },

    /// 将输入追加到指定的已有会话
    SendToSession {
        session_id: String,
        input: String,
    },

    /// 取消按钮按下
    ///
    /// 用户点击取消按钮时触发，用于中断正在进行的 Agent 响应
    CancelPressed,

    /// 自动模型选择开关切换
    ///
    /// 启用/禁用自动选择最佳模型的功能
    AutoModelToggled(bool),
    ReasoningEffortSelected(Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>),
    AcpAgentSelected(Option<String>),
    SessionAgentSelected(Option<String>),
    AcpHistoryModeSelected(crate::app::state::AcpHistoryReplayMode),
    AcpHistoryRecentCountChanged(String),
    /// 切换当前父会话的 AgentTool 子会话弹层。
    ToggleAgentSessionsPopover,
    /// 关闭当前父会话的 AgentTool 子会话弹层。
    CloseAgentSessionsPopover,
    /// 关闭弹层并打开真实持久化子会话。
    OpenAgentSession {
        project_path: String,
        session_id: String,
    },
    /// 请求停止子会话所属的真实 plan-run。
    StopPlanRun(String),
    /// plan-run 停止请求完成；实时状态仍由 SSE cancelled 事件收敛。
    StopPlanRunFinished {
        task_id: String,
        result: Result<vw_gateway_client::vw_api_types::task::TaskPlanRunStatusResponse, String>,
    },

    /// 模型被选择
    ///
    /// 用户手动选择特定的 AI 模型
    ModelSelected(String),

    /// 模型输入框内容变化
    ///
    /// 用于手工填写列表外的模型标识符
    ModelInputChanged(String),

    /// 切换会话级工具分桶
    SessionToolBucketToggled(crate::app::state::SessionToolBucket),

    /// 切换会话工具弹窗标签
    SessionToolSelectorTabSelected(crate::app::state::SessionToolSelectorTab),

    /// 从底部栏打开会话工具弹窗并选中指定 TAB；同一 TAB 再点则关闭。
    OpenSessionToolSelectorTab(crate::app::state::SessionToolSelectorTab),

    /// 修改会话工具弹窗搜索关键词
    SessionToolSelectorSearchChanged(String),

    /// 切换会话技能目录筛选
    SessionToolSelectorSkillDirectoryScopeChanged(crate::app::state::SkillsDirectoryScope),

    /// 切换会话工具分组折叠状态
    SessionToolGroupCollapsedToggled(crate::app::state::SessionToolGroup),

    /// 切换会话工具分组的整组启用状态
    SessionToolGroupToolsToggled(crate::app::state::SessionToolGroup),

    /// 当前会话工具全选
    SessionToolSelectorSelectAll,

    /// 当前会话工具反选
    SessionToolSelectorInvert,

    /// 切换单个会话工具
    SessionToolToggled(String),

    /// 切换手动工具上下文选中状态
    SessionManualToolSelected(String),

    /// 切换技能上下文选中状态
    SessionSkillSelected(String),

    /// 重置会话级工具分桶
    SessionToolSelectorReset,

    /// 任务模式开关切换
    ///
    /// 启用/禁用任务模式（Task Mode），任务模式用于复杂的多步骤任务
    TaskModeToggled(bool),

    /// 工作流模式开关切换
    ///
    /// 启用后，本轮聊天会先生成临时 Dify 工作流，再通过 workflow 流式执行。
    WorkflowModeToggled(bool),

    /// 目标模式已固定开启；消息保留仅为兼容，处理端会忽略关闭请求。
    GoalModeToggled(bool),

    /// 任务优先级变化
    ///
    /// 修改当前任务的优先级
    TaskModePriorityChanged(String),

    /// 任务 ACP 智能体变化
    ///
    /// 切换任务模式使用的 ACP 智能体
    TaskModeExecutorChanged(Option<String>),

    /// 切换任务模式使用的推理强度
    TaskModeReasoningEffortChanged(Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>),

    /// 任务模式模型变化
    ///
    /// 支持手工填写任务模式使用的模型标识符
    TaskModeModelChanged(String),

    /// 任务模式任务拆分策略变化
    ///
    /// 控制当前任务是否跟随系统设置、强制开启或关闭任务拆分
    TaskModeTaskSplitModeChanged(crate::app::task::TaskSplitMode),

    /// 任务模式子任务内容变化
    ///
    /// 修改指定索引的子任务描述
    TaskModeSubtaskChanged {
        /// 子任务索引
        index: usize,
        /// 新的子任务内容
        value: String,
    },

    /// 任务模式子任务编辑器动作
    ///
    /// 对子任务文本编辑器执行的操作（如光标移动、选择等）
    TaskModeSubtaskEditorAction {
        /// 子任务索引
        index: usize,
        /// 编辑器动作
        action: text_editor::Action,
    },

    /// 添加子任务
    ///
    /// 在任务列表中添加新的子任务
    TaskModeAddSubtask,

    /// 移除子任务
    ///
    /// 删除指定索引的子任务
    TaskModeRemoveSubtask(usize),

    /// 上移子任务
    ///
    /// 将指定索引的子任务向上移动一位
    TaskModeMoveSubtaskUp(usize),

    /// 下移子任务
    ///
    /// 将指定索引的子任务向下移动一位
    TaskModeMoveSubtaskDown(usize),

    /// 从队列中移除消息
    ///
    /// 删除待发送队列中指定索引的消息
    QueueRemove(usize),

    /// 再次编辑队列消息
    ///
    /// 从待发送队列移除指定消息，并仅将文字回填到输入框
    QueueEdit(usize),

    /// 将队列消息提升为引导
    ///
    /// 把指定消息移至队首，并在当前请求的最近安全交接点优先应用
    QueueGuide(usize),

    /// 队列消息上移
    ///
    /// 将待发送队列中的消息向上移动
    QueueUp(usize),

    /// 队列消息下移
    ///
    /// 将待发送队列中的消息向下移动
    QueueDown(usize),

    /// 复制队列消息内容
    ///
    /// 将待发送队列中指定消息的完整内容写入剪贴板
    QueueCopy(usize),

    /// 提交时钟滴答
    ///
    /// 定时触发消息提交检查
    SubmitTick,

    /// Agent 流式响应增量更新
    ///
    /// 接收 Agent 响应的增量文本内容
    AgentStreamDelta(u64, String),

    /// Agent 工具调用的类型化生命周期事件。
    AgentToolLifecycle(u64, vw_gateway_client::GatewayChatToolEvent),

    /// 自动压缩生命周期状态；`completed` 为真时表示安全替换已经完成。
    AgentCompactionStatus(u64, bool),

    /// SessionTool 创建完成后，从网关读取到的权威可见任务会话元信息。
    VisibleTaskSessionLoaded(Result<vw_shared::session::info::Info, String>),

    /// 合并一小段时间内的 Agent 流式增量，降低 UI 重绘和会话保存频率。
    AgentStreamDeltaFlush(String),

    /// 保存流式会话的最新消息增量。
    AgentStreamSessionSaveFlush(String),

    /// 流式消息增量的异步回执。
    AgentStreamSessionDeltaSaved {
        session_id: String,
        generation: u64,
        revision: u64,
        updated_ms: u64,
        result: Result<(), crate::app::session_gateway::SessionMessageDeltaSaveError>,
    },

    /// 系统提示词实际加载的协议文件。
    AgentContextLoaded(u64, Vec<String>),

    /// Workflow 节点卡片更新
    ///
    /// 接收 workflow 流式执行中的节点状态卡片。
    AgentWorkflowNodeUpdate(u64, String),

    /// plan-run 已创建规划或执行子会话，桌面应在后台登记它。
    PlanRunSessionStarted {
        parent_request_id: u64,
        parent_session_id: String,
        session_id: String,
        title: String,
        directory: String,
        query: String,
    },

    /// plan-run 规划或执行子会话的普通聊天增量。
    PlanRunSessionDelta {
        session_id: String,
        delta: String,
    },

    /// plan-run 子会话正在等待递归后代完成，实时态应保持活跃。
    PlanRunSessionWaiting(String),

    /// plan-run 规划或执行子会话已结束实时输出。
    PlanRunSessionFinished(String),

    /// plan-run 规划或执行子会话已被取消。
    PlanRunSessionCancelled(String),

    /// 切换聊天面板的对话／轨迹主体。
    ChatPanelModeSelected(crate::app::state::ChatPanelMode),

    /// 更新轨迹账本搜索文本。
    TrajectorySearchChanged(String),

    /// 在真实耗时与等宽时间线间切换。
    TrajectoryDurationToggled,

    /// 按全局规则展开或折叠所有轮次。
    TrajectoryTurnsToggled,

    /// 按全局规则展开或折叠所有工具调用步骤。
    TrajectoryCallsToggled,

    /// 独立展开或折叠一个轮次。
    TrajectoryTurnToggled(u32),

    /// 独立展开或折叠一个工具调用步骤。
    TrajectoryToolStepToggled(u32),

    /// 选中一条稳定轨迹记录。
    TrajectoryRecordSelected(String),

    /// 切换轨迹详情标签。
    TrajectoryDetailsTabSelected(crate::app::state::TrajectoryDetailsTab),

    /// 关闭轨迹详情。
    TrajectoryDetailsClosed,

    /// 在只读轨迹详情中移动光标或更新选区。
    TrajectoryDetailsEditorAction(usize, text_editor::Action),

    /// 在指定详情字段上打开右键复制菜单。
    TrajectoryDetailsContextMenuOpened {
        index: usize,
        x: f32,
        y: f32,
    },

    /// 关闭轨迹详情的右键菜单。
    TrajectoryDetailsContextMenuClosed,

    /// 复制轨迹详情的当前选区，未选中时复制整个字段。
    TrajectoryDetailsContextMenuCopy,

    /// 展开或折叠当前详情标签中的 JSON 路径。
    TrajectoryDetailsJsonToggled(String),

    /// 复制 JSON 树节点或完整树。
    TrajectoryDetailsJsonCopied(serde_json::Value),

    /// 延迟后重试补载某一步骤的原始 LLM 载荷。
    TrajectoryRawRetry {
        session_id: String,
        turn_id: String,
        step_index: u32,
        attempt: u8,
    },

    /// 某一步骤的原始 LLM 载荷补载完成。
    TrajectoryRawLoaded {
        session_id: String,
        turn_id: String,
        step_index: u32,
        attempt: u8,
        result: Result<Option<serde_json::Value>, String>,
    },

    /// 进入轨迹模式时主动刷新的会话 UI 快照。
    TrajectorySnapshotLoaded {
        session_id: String,
        result: Result<Option<vw_gateway_client::SessionUiLoadResult>, String>,
    },

    /// Agent 步骤开始
    ///
    /// 标记 Agent 处理流程中的一个步骤开始执行
    AgentStepStart(u64, String, u32, u64, Option<String>),

    /// Agent 步骤完成
    ///
    /// 标记 Agent 处理流程中的一个步骤执行完成，包含 Token 使用统计
    AgentStepFinish(u64, String, u32, u64, models::TokenUsage, Option<String>, Option<String>),

    /// Agent 步骤成本加载完成
    ///
    /// 异步加载的步骤成本信息已就绪
    AgentStepCostLoaded(u64, String, u32, Option<String>, Option<f64>),

    /// Agent 工具轮次执行完成
    ///
    /// 标记本轮工具执行已结束，下一轮模型请求尚未开始。
    AgentPostToolRound(u64, String, u32),

    /// 会话保存完成
    ///
    /// 用于接收 WASM 端异步保存的回执，不触发额外状态更新。
    SessionSaveAck,

    /// 会话保存永久失败
    ///
    /// 通过现有 toast 组件向用户暴露无法自动恢复的保存错误。
    SessionSaveFailed(String),

    /// Agent 流式响应完成
    ///
    /// Agent 的完整响应已接收完毕
    AgentStreamDone(u64, models::TokenUsage, Option<String>, Option<String>),

    /// plan-run 的父请求已被取消，不应按成功或错误收尾。
    AgentStreamCancelled(u64),

    /// Agent 流式响应错误
    ///
    /// Agent 响应过程中发生错误
    AgentStreamError(u64, String),

    /// 会话标题生成完成
    ///
    /// 自动生成的会话标题已就绪
    SessionTitleGenerated(String, String),

    /// 问题轮询时钟滴答
    ///
    /// 定时检查问题对话框状态
    QuestionPollTick,

    /// 权限轮询时钟滴答
    ///
    /// 定时检查待处理的权限批准请求
    PermissionPollTick,

    /// 触发输入面板 Todo 列表加载
    ///
    /// 从当前活跃会话异步加载 Todo 列表并写回应用状态。
    LoadInputPanelTodos,

    /// Todo 轮询时钟滴答
    ///
    /// 定时刷新当前活跃会话的 Todo 列表
    TodoPollTick,

    /// 问题列表加载完成
    QuestionListLoaded(Result<Vec<vw_shared::question::Request>, String>),

    /// 权限列表加载完成
    PermissionListLoaded(Result<Vec<vw_gateway_client::PendingPermissionRequestDto>, String>),

    /// 输入面板 Todo 列表加载完成
    InputPanelTodosLoaded(String, Result<Vec<vw_shared::todo::Todo>, String>),

    /// 问题回复提交完成
    QuestionReplySubmitted(Result<(), String>),

    /// 问题拒绝提交完成
    QuestionRejected(Result<(), String>),

    /// 权限回复提交完成
    PermissionReplySubmitted(Result<(), String>),

    /// 问题选项切换
    ///
    /// 用户切换问题选项的选中状态
    QuestionOptionToggled(usize, String),

    /// 问题自定义回答内容变化
    ///
    /// 用户修改自定义回答输入框的内容
    QuestionCustomChanged(usize, String),

    /// 提交问题答案
    ///
    /// 用户确认提交对 Agent 提问的回答
    QuestionSubmit,

    /// 拒绝回答问题
    ///
    /// 用户拒绝回答 Agent 的提问
    QuestionReject,

    /// 仅本次批准权限请求
    PermissionApproveOnce,

    /// 永久批准权限请求
    PermissionApproveAlways,

    /// 永久批准当前所有待处理权限请求
    PermissionApproveAllAlways,

    /// 拒绝权限请求
    PermissionReject,

    /// 切换当前显示的权限请求
    PermissionSelectRequest(String),

    /// 切换完全访问权限自动批准
    ToggleFullAccessPermission,

    /// 输入编辑器动作
    ///
    /// 对主输入框文本编辑器执行的操作
    InputEditorAction(text_editor::Action),

    /// wasm IME 已提交文本
    ///
    /// 隐藏 textarea 的 input 事件提交的最终文本。
    WasmImeCommit(String),

    /// wasm 输入桥请求聚焦隐藏 textarea。
    WasmImeFocus,

    /// wasm 输入桥请求删除前一个字符。
    WasmImeBackspace,

    /// wasm 输入桥请求删除后一个字符。
    WasmImeDelete,

    /// wasm 输入桥请求移动或选择文本。
    WasmImeMove {
        motion: text_editor::Motion,
        select: bool,
    },

    /// 打开输入框右键菜单
    OpenInputContextMenu {
        /// 菜单锚点 X 坐标
        x: f32,
        /// 菜单锚点 Y 坐标
        y: f32,
    },

    /// 关闭输入框右键菜单
    CloseInputContextMenu,

    /// 复制输入框选中文本
    CopyInputSelection,

    /// 剪切输入框选中文本
    CutInputSelection,

    /// 粘贴到输入框
    PasteIntoInput,

    /// 智能粘贴结果已就绪
    ClipboardPasteResolved(ClipboardPastePayload),

    /// 全选输入框文本
    SelectAllInput,

    /// 消息编辑器动作
    ///
    /// 对历史消息进行编辑时的编辑器操作
    MessageEditorAction(usize, text_editor::Action),

    /// 特殊消息文本编辑器动作
    ///
    /// 对带思考块或工具块消息中的普通文本片段执行只读编辑器操作
    SpecialTextEditorAction(usize, usize, text_editor::Action),

    /// 工具卡片文本编辑器动作
    ///
    /// 对工具卡片内部的文本片段执行只读编辑器操作
    ToolTextEditorAction(usize, usize, usize, text_editor::Action),

    /// Markdown 块编辑器动作
    ///
    /// 对助手正文 Markdown 块（消息索引 + 文本块索引 + 块索引）执行只读编辑器操作
    MarkdownBlockEditorAction(usize, usize, usize, text_editor::Action),

    /// 思考块编辑器动作
    ///
    /// 对消息中思考块内容进行编辑时的编辑器操作
    ThinkEditorAction(usize, usize, text_editor::Action),

    /// 切换思考块展开/折叠状态
    ///
    /// 展开或折叠指定消息中的思考块
    ToggleThink(usize, usize, bool),

    /// 思考块悬停进入
    ///
    /// 鼠标进入思考块区域
    ThinkHover(usize, usize),

    /// 思考块悬停离开
    ///
    /// 鼠标离开思考块区域
    ThinkHoverLeave,

    /// 切换工具卡片内文件项的展开状态
    ToggleToolFile(usize, usize, String),
    ToggleTool(usize, usize),

    /// 工具内文件项悬停进入
    ToolFileHover(String),

    /// 工具内文件项悬停离开
    ToolFileHoverLeave,

    /// 工具调用悬停进入
    ///
    /// 鼠标进入工具调用区域
    ToolHover(usize, usize),

    /// 工具调用悬停离开
    ///
    /// 鼠标离开工具调用区域
    ToolHoverLeave,
    ToggleExploreSummary(usize, usize),
    OpenToolDetail(usize, usize, String),
    ToolDetailEditorAction(text_editor::Action),
    ToolDetailOpenContextMenu {
        x: f32,
        y: f32,
    },
    ToolDetailCloseContextMenu,
    ToolDetailContextMenuCopy,
    ToolDetailContextMenuCut,
    ToolDetailContextMenuPaste,
    ToolDetailContextMenuDelete,
    ToolDetailEditorWheelScrolled {
        delta: iced::mouse::ScrollDelta,
        viewport_height: f32,
    },
    ToolDetailScrollbarChanged {
        top_line: f32,
        viewport_height: f32,
    },
    CloseToolDetail,

    /// 切换待办事项面板
    ///
    /// 显示或隐藏待办事项面板
    ToggleTodoPanel,

    /// 设置待办事项面板显示位置。
    SetTodoPanelPlacement(TodoPanelPlacement),

    /// 待办事项动画时钟滴答
    ///
    /// 驱动待办事项面板展开/折叠动画
    TodoAnimTick,

    /// 滚动位置变化
    ///
    /// 聊天消息列表的滚动位置发生改变
    ScrollChanged {
        /// 垂直相对偏移量
        offset_y: f32,
        /// 垂直绝对偏移量（像素）
        absolute_offset_y: f32,
        /// 视口高度
        viewport_h: f32,
    },

    /// 已渲染聊天消息的真实布局测量结果
    VisibleMessageMeasurements(Vec<ChatMessageMeasurement>),

    /// 滚动到底部
    ///
    /// 将聊天消息列表滚动到最底部（最新消息）
    ScrollToBottom,

    /// 根据消息 ID 定位到聊天中的目标消息
    LocateChatMessage(String),

    /// 根据消息索引定位到聊天中的目标消息
    LocateChatMessageIndex(usize),

    /// 切换聊天面板全屏状态
    ToggleFullscreen,

    /// 切换聊天面板半全屏状态
    ToggleHalfFullscreen,

    /// 鼠标进入聊天右上角全屏控件区域
    FullscreenOverlayEntered,

    /// 鼠标离开聊天右上角全屏控件区域
    FullscreenOverlayExited,

    /// 打开聊天消息右键菜单
    ///
    /// 记录目标消息与菜单位置，用于展示文本操作菜单
    OpenMessageContextMenu {
        /// 目标元素键
        target: u64,
        /// 菜单锚点 X 坐标
        x: f32,
        /// 菜单锚点 Y 坐标
        y: f32,
        /// 当前消息的可操作文本
        text: String,
    },

    /// 打开聊天消息右键菜单；操作发生时再解析消息文本。
    OpenChatMessageContextMenu {
        target: u64,
        x: f32,
        y: f32,
        session_id: String,
        message_idx: usize,
    },

    /// 关闭聊天消息右键菜单
    CloseMessageContextMenu,

    /// 复制右键菜单文本
    CopyContextMenuText,

    /// 将右键菜单文本添加到输入框
    AppendContextMenuText,

    /// 使用百度搜索右键菜单文本
    SearchContextMenuWithBaidu,

    /// 使用 Google 搜索右键菜单文本
    SearchContextMenuWithGoogle,

    /// 使用 Bing 搜索右键菜单文本
    SearchContextMenuWithBing,

    ToggleResetMenu(usize),

    CloseResetMenu,

    OpenForkSessionDialog(usize),

    CloseForkSessionDialog,

    ForkSessionAt {
        msg_idx: usize,
        target: ForkSessionTarget,
    },

    MessageIdsRecoveredForAction {
        session_id: String,
        msg_idx: usize,
        action: MessageIndexedSessionAction,
        result: Result<Vec<Option<String>>, String>,
    },

    ForkSessionFinished {
        result: Result<ForkSessionLoaded, String>,
    },

    LocalForkSessionCreated {
        result: Result<(vw_shared::session::info::Info, Option<String>), String>,
        target: ForkSessionTarget,
        retained_chat: Vec<crate::app::models::ChatMessage>,
        retained_message_ids: Vec<Option<String>>,
    },

    ResetSessionToMessage {
        msg_idx: usize,
        revert_code: bool,
    },

    ResetSessionFinished {
        result: Result<vw_shared::session::info::Info, String>,
        session_id: String,
        retained_chat: Vec<crate::app::models::ChatMessage>,
        retained_message_ids: Vec<Option<String>>,
        directory: Option<String>,
    },

    /// 插入位置选择
    ///
    /// 用户选择了插入文件引用的位置
    InsertPosition(String, usize, usize),

    /// 插入当前匹配项
    ///
    /// 插入当前高亮的匹配项
    InsertActiveMatch,

    /// 插入选中范围
    ///
    /// 插入选中的文本范围
    InsertSelectionRange,

    /// 插入选中位置列表
    ///
    /// 插入多个选中的位置
    InsertSelectionPositions,

    /// 确认插入选择
    ///
    /// 确认并执行插入操作
    InsertSelected,

    /// 追加文本
    ///
    /// 在输入框末尾追加文本内容
    AppendText(String),

    /// 文件搜索输入变化
    ///
    /// 用户修改文件搜索框的输入内容
    FileSearchInputChanged(String),

    /// 文件搜索选择
    ///
    /// 用户从搜索结果中选择一个文件
    FileSearchSelect(String),

    /// 文件搜索导航上移
    ///
    /// 在文件搜索结果中向上移动光标
    FileSearchNavigateUp,

    /// 文件搜索导航下移
    ///
    /// 在文件搜索结果中向下移动光标
    FileSearchNavigateDown,

    /// 文件搜索选择当前项
    ///
    /// 选择当前高亮的文件搜索结果
    FileSearchSelectCurrent,

    /// Slash 面板关闭
    SlashPaletteClosed,

    /// Slash 面板向上导航
    SlashPaletteNavigateUp,

    /// Slash 面板向下导航
    SlashPaletteNavigateDown,

    /// Slash 面板选择当前高亮项
    SlashPaletteSelectCurrent,

    /// Slash 面板选择指定条目
    SlashPaletteSelect {
        kind: crate::app::state::SlashPaletteKind,
        id: String,
    },

    /// Slash 面板展开某个类别
    SlashPaletteExpandKind(crate::app::state::SlashPaletteKind),

    /// 输入区域拖放
    ///
    /// 文件被拖放到输入区域
    InputAreaDragDrop,

    /// 输入区域拖拽悬停状态变化
    ///
    /// 文件被拖入或拖出输入区域
    InputAreaDragHoverChanged(bool),

    /// 工具文件过滤器变化
    ///
    /// 修改工具调用结果中文件列表的过滤条件
    ToolFilesFilterChanged(String),

    /// 移除文件引用
    ///
    /// 从输入中移除指定的文件引用
    RemoveFileReference(String),

    /// 文件引用悬停状态变化
    ///
    /// 鼠标进入或离开文件引用标记
    FileReferenceHoverChanged(Option<usize>),
}

/// 处理聊天消息并更新应用状态
///
/// 这是聊天模块的核心消息处理函数，负责：
/// 1. 根据消息类型路由到对应的子模块处理函数
/// 2. 处理流式响应的特殊逻辑（思考块状态同步）
/// 3. 同步消息编辑器状态
///
/// # 参数
///
/// - `app`：应用状态的可变引用，包含所有聊天相关的数据
/// - `message`：要处理的聊天消息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，可能包含需要执行的异步操作或 UI 命令
///
/// # 消息路由策略
///
/// 消息被分为四大类，由对应的子模块处理：
///
/// - **输入类**（`input::update`）：处理用户界面交互，如文本输入、悬停状态、滚动等
/// - **流式响应类**（`stream::update`）：处理 Agent 的实时响应流，包括增量更新和步骤状态
/// - **上下文插入类**（`context::update`）：处理文件引用和代码片段的插入操作
/// - **会话管理类**（`session::update`）：处理消息发送、取消、队列管理、模型选择等
///
/// # 流式响应的特殊处理
///
/// 对于 `AgentStreamDelta` 消息，函数会额外执行以下操作：
///
/// 1. 读取流式尾状态中的思考块数量和开闭状态
/// 2. 跟踪当前正在展开的思考块索引
/// 3. 清理已删除思考块的编辑器状态
/// 4. 自动将思考块滚动到底部以显示最新内容
///
/// # 示例
///
/// ```ignore
/// let task = update(&mut app, ChatMessage::InputChanged("Hello".to_string()));
/// // task 可能包含 UI 更新命令
/// ```
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    // 检查是否为流式增量消息，用于后续特殊处理
    let is_stream_delta = matches!(
        message,
        ChatMessage::AgentStreamDelta(_, _)
            | ChatMessage::AgentToolLifecycle(_, _)
            | ChatMessage::AgentCompactionStatus(_, _)
            | ChatMessage::VisibleTaskSessionLoaded(_)
            | ChatMessage::AgentStreamDeltaFlush(_)
    );

    // 根据消息类型路由到对应的子模块处理函数
    let mut task = match message {
        // 输入相关消息：文本编辑、悬停状态、滚动、文件搜索、任务模式输入等
        ChatMessage::InputChanged(_)
        | ChatMessage::QuickMessageSelected(_)
        | ChatMessage::QuickMessageGroupChanged(_)
        | ChatMessage::ToggleShijuSkillMenu
        | ChatMessage::SelectShijuSkill(_)
        | ChatMessage::InputEditorAction(_)
        | ChatMessage::WasmImeCommit(_)
        | ChatMessage::WasmImeFocus
        | ChatMessage::WasmImeBackspace
        | ChatMessage::WasmImeDelete
        | ChatMessage::WasmImeMove { .. }
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
        | ChatMessage::OpenAgentSession { .. }
        | ChatMessage::StopPlanRun(_)
        | ChatMessage::StopPlanRunFinished { .. } => input::update(app, message),

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
        | ChatMessage::TrajectorySnapshotLoaded { .. } => trajectory::update(app, message),

        // 流式响应相关消息：Agent 增量更新、步骤状态、会话标题、问题应答等
        ChatMessage::AgentStreamDelta(_, _)
        | ChatMessage::AgentToolLifecycle(_, _)
        | ChatMessage::AgentCompactionStatus(_, _)
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
        | ChatMessage::SessionSaveAck
        | ChatMessage::SessionSaveFailed(_)
        | ChatMessage::AgentStreamDone(_, _, _, _)
        | ChatMessage::AgentStreamCancelled(_)
        | ChatMessage::AgentStreamError(_, _)
        | ChatMessage::SessionTitleGenerated(_, _)
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
        | ChatMessage::PermissionReplySubmitted(_) => stream::update(app, message),

        // 上下文插入相关消息：文件路径、代码片段、位置选择等
        ChatMessage::InsertPosition(_, _, _)
        | ChatMessage::InsertActiveMatch
        | ChatMessage::InsertSelectionRange
        | ChatMessage::InsertSelectionPositions
        | ChatMessage::InsertSelected
        | ChatMessage::AppendText(_) => context::update(app, message),

        // 会话管理相关消息：发送、取消、模型选择、队列操作等
        ChatMessage::SendPressed
        | ChatMessage::SendToSession { .. }
        | ChatMessage::CompactSessionRequested { .. }
        | ChatMessage::CompactSessionConfirmationResolved { .. }
        | ChatMessage::CompactSessionFinished { .. }
        | ChatMessage::GenerateGitCommitMessage { .. }
        | ChatMessage::CancelPressed
        | ChatMessage::AutoModelToggled(_)
        | ChatMessage::ReasoningEffortSelected(_)
        | ChatMessage::AcpAgentSelected(_)
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
        | ChatMessage::MessageIdsRecoveredForAction { .. }
        | ChatMessage::ForkSessionFinished { .. }
        | ChatMessage::LocalForkSessionCreated { .. }
        | ChatMessage::ResetSessionToMessage { .. }
        | ChatMessage::ResetSessionFinished { .. } => session::update(app, message),
    };

    if app.show_agent_sessions_popover
        && crate::app::components::input_panel::agent_sessions::running_agent_sessions(app)
            .is_empty()
    {
        app.show_agent_sessions_popover = false;
    }

    // 对流式增量消息执行特殊处理：同步思考块状态
    if is_stream_delta {
        let stream_tail = app.active_session_id.as_deref().and_then(|session_id| {
            let state = &app.get_session_runtime(session_id).stream_tail_state;
            state
                .message_idx()
                .map(|message_idx| (message_idx, state.think_count(), state.thinking_open()))
        });
        if let Some((i, think_count, thinking_open)) = stream_tail {
            // 计算当前应该展开的思考块索引
            // 如果有正在思考的块且存在思考块，则展开最后一个
            let open_idx = if thinking_open && think_count > 0 {
                Some(think_count.saturating_sub(1))
            } else {
                None
            };

            // 获取之前的状态，用于检测变化
            let prev_open_idx = if app.chat_stream_think_msg_idx == Some(i) {
                app.chat_stream_think_open_idx
            } else {
                None
            };
            let prev_count = if app.chat_stream_think_msg_idx == Some(i) {
                app.chat_stream_think_count
            } else {
                0
            };

            // 更新当前跟踪的消息索引和思考块数量
            if app.chat_stream_think_msg_idx != Some(i) {
                app.chat_stream_think_msg_idx = Some(i);
            }
            app.chat_stream_think_count = think_count;
            app.chat_stream_think_open_idx = open_idx;

            // 清理已删除思考块的编辑器状态
            // 如果思考块数量减少，移除多余的编辑器
            if prev_count > think_count {
                for think_idx in think_count..prev_count {
                    let key = think_block_key(i, think_idx);
                    app.chat_think_editors.remove(&key);
                    app.chat_think_expanded.remove(&key);
                    app.chat_think_collapsed.remove(&key);
                }
            }

            let think_state_changed = prev_open_idx != open_idx || prev_count != think_count;

            if think_state_changed
                && open_idx.is_some()
                && let Some(scroll_task) = throttled_stream_autoscroll_task(app)
            {
                task = task.chain(scroll_task);
            }
        }
    } else {
        let (visible_start_idx, visible_end_idx) = app.visible_chat_message_window();
        let pending_chunk_starts =
            app.pending_chat_message_editors_window_chunks(visible_start_idx, visible_end_idx);
        if !pending_chunk_starts.is_empty()
            && let Some(session_id) = app.active_session_id.clone()
        {
            app.mark_chat_ui_chunks_preparing(&pending_chunk_starts);
            task = task.chain(message::project::prepare_session_ui_chunks_task(
                session_id,
                app.active_shared_chat_messages(),
                pending_chunk_starts,
                None,
                app.dialogue_flow_show_reasoning_summary,
            ));
        }
    }

    task
}
#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;
#[cfg(test)]
mod tests;
