use iced::widget::text_editor;
use vw_api_agent_scheduler::AgentSchedulerTaskDetailDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Assignee,
    Priority,
    StartDate,
    DueDate,
    PullRequest,
    Properties,
    Labels,
}

#[derive(Debug, Clone)]
pub enum Event {
    Reload,
    EditComment(text_editor::Action),
    ReplyTo(Option<String>),
    PostComment,
    EditField(Option<Field>),
    FieldInput(String),
    SaveField,
    ClearField,
    UnlinkPullRequest(String),
    ToggleActivity,
    ShowExecution,
    ToggleSubtasks,
    ToggleRun(String),
    Loaded { request: u64, result: Result<AgentSchedulerTaskDetailDto, String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Load,
    PostComment,
    SaveField,
    Unlink,
}

#[derive(Default)]
pub struct State {
    pub project: String,
    pub task_id: String,
    pub data: Option<AgentSchedulerTaskDetailDto>,
    pub request: Option<(u64, Operation)>,
    pub load_failed: bool,
    pub comment: text_editor::Content,
    pub comment_id: Option<String>,
    pub reply_to: Option<String>,
    pub editor: Option<Field>,
    pub input: String,
    pub activity_expanded: bool,
    pub subtasks_collapsed: bool,
    pub expanded_run: Option<String>,
}
