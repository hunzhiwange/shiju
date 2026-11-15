use desktop_session::message::chat::request::ChatSendBehavior;
use desktop_task_model::models::TaskSplitMode;
use vw_api_chat_identity::ReasoningEffortDto;
#[derive(Debug, Clone)]
pub enum Message {
 SelectChatSendBehavior(ChatSendBehavior), ToggleSendModePopover, CloseSendModePopover,
 ToggleFullAccessPermission, WorkflowModeToggled(bool), SendPressed, CancelPressed,
 #[cfg(not(target_arch = "wasm32"))]
 VoiceChatStart,
 AddTaskFromInput(String),
 AddTaskFromInputWithOptions { content: String, priority: String, model: String, reasoning_effort: Option<ReasoningEffortDto>, task_split_mode: TaskSplitMode, subtasks: Vec<String> },
}
