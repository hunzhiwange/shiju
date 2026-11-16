//! 从现有会话运行状态准备发送；跨领域任务创建由宿主执行。
use super::request::{ChatSendBehavior, QueueItem};
use super::session::{
    InlineModeCommand, compose_query_with_attachments, compose_query_with_session_context,
    parse_inline_mode_command, request_delegate_agent,
};
use crate::session::agent::SessionRuntimeState;
use vw_api_acp_model::acp_model_source::AcpModelSource;

pub enum SendInputEffect {
    None,
    InlineMode,
    AddTask {
        content: String,
        priority: String,
        model: String,
        reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
        task_split_mode: desktop_task_model::TaskSplitMode,
        subtasks: Vec<String>,
    },
    Send(QueueItem),
}

pub fn apply_inline_mode_command<R>(
    runtime: &mut SessionRuntimeState<R>,
    command: InlineModeCommand,
) {
    runtime.task_mode_enabled = command == InlineModeCommand::TaskMode;
    runtime.input_editor = iced::widget::text_editor::Content::new();
}

pub fn prepare_send_input<R>(
    runtime: &mut SessionRuntimeState<R>,
    files: &mut Vec<String>,
    root: Option<String>,
    allowed_tools: Option<Vec<String>>,
    plan_run_child: bool,
    send_behavior: ChatSendBehavior,
    created_ms: u64,
) -> SendInputEffect {
    let query = runtime.input_editor.text().to_string();
    let selected_tools = runtime.tool_selector.selected_manual_tools();
    let selected_skills = runtime.tool_selector.selected_manual_skills();
    let selected_shiju_skills = runtime.tool_selector.selected_shiju_skills();
    if query.trim().is_empty()
        && files.is_empty()
        && selected_tools.is_empty()
        && selected_skills.is_empty()
        && selected_shiju_skills.is_empty()
    {
        return SendInputEffect::None;
    }
    let workflow_mode_enabled = runtime.workflow_mode_enabled && !plan_run_child;
    let goal_mode_enabled = !plan_run_child;
    if let Some(command) = parse_inline_mode_command(&query) {
        apply_inline_mode_command(runtime, command);
        return SendInputEffect::InlineMode;
    }
    let attachments = std::mem::take(files);
    let query = compose_query_with_session_context(
        &query,
        &selected_tools,
        &selected_skills,
        &selected_shiju_skills,
    );
    runtime.tool_selector.clear_manual_context_selection();
    if runtime.task_mode_enabled {
        return SendInputEffect::AddTask {
            content: compose_query_with_attachments(&query, &attachments),
            priority: runtime.task_mode_priority.clone(),
            model: runtime.task_mode_model.clone(),
            reasoning_effort: runtime.task_mode_reasoning_effort,
            task_split_mode: runtime.task_mode_task_split_mode,
            subtasks: runtime.task_mode_subtasks.clone(),
        };
    }
    let acp_model_source = runtime.acp_model_source;
    let model = if runtime.auto_model
        || (runtime.acp_agent.is_some() && acp_model_source == AcpModelSource::CliLocal)
    {
        None
    } else {
        Some(runtime.model.clone())
    };
    let reasoning_effort = runtime.reasoning_effort;
    let acp_agent = runtime.acp_agent.clone();
    let agent = request_delegate_agent(runtime.agent.clone());
    let acp_test = acp_agent.is_some();
    let acp_allowed_tools = acp_test.then(|| allowed_tools.clone()).flatten();
    let acp_history_mode = runtime.acp_history_mode;
    let acp_recent_count = runtime.acp_recent_count.clamp(1, 20);
    let full_access_enabled = runtime.full_access_enabled;
    let acp_force_new_session = acp_agent.is_some()
        && (runtime.acp_rebuild_required
            || runtime.last_effective_acp_agent.as_ref() != acp_agent.as_ref());
    tracing::debug!(
        target: "vw_desktop",
        acp_test,
        acp_agent = ?acp_agent,
        acp_force_new_session,
        acp_history_mode = %acp_history_mode.as_str(),
        acp_recent_count,
        model = ?model,
        agent = ?agent,
        allowed_tools = ?allowed_tools,
        has_root = root.is_some(),
        query_len = query.len(),
        "desktop chat request prepared"
    );
    SendInputEffect::Send(QueueItem {
        created_ms,
        query,
        attachments,
        root,
        model,
        reasoning_effort,
        acp_test,
        acp_model_source,
        acp_agent,
        acp_allowed_tools,
        agent,
        allowed_tools,
        require_agent_tool: false,
        acp_force_new_session,
        acp_history_mode,
        acp_recent_count,
        full_access_enabled,
        send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled,
        goal_mode_enabled,
    })
}
