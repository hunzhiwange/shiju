use iced::widget::text_editor;
use vw_api_agent_scheduler::{
    AgentSchedulerTaskAgentTurnStatus, AgentSchedulerTaskDetailDto, AgentSchedulerTaskReplyMode,
    AgentSchedulerTaskReplySession,
};

pub const COMMENT_EDITOR_ID: &str = "task-detail-comment";
pub const ATTACHMENT_INPUT_ID: &str = "task-detail-attachment";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Assignee,
    Priority,
    StartDate,
    DueDate,
    PullRequest,
    Properties,
    Labels,
    Attachment,
}

#[derive(Debug, Clone)]
pub enum Event {
    Reload,
    PollReplies(u64),
    ExpandComment,
    CollapseComment,
    EditComment(text_editor::Action),
    ReplyTo(Option<String>),
    PostComment,
    StopReply(String),
    ToggleReplyMenu,
    CloseReplyMenu,
    SelectReplyMode(Option<AgentSchedulerTaskReplyMode>),
    AddAttachment,
    RemoveAttachment(usize),
    EditField(Option<Field>),
    FieldInput(String),
    SaveField,
    ClearField,
    UnlinkPullRequest(String),
    ToggleActivity,
    ShowExecution,
    ToggleRun(String),
    Loaded { request: u64, result: Result<AgentSchedulerTaskDetailDto, String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Load,
    PollReplies,
    PostComment,
    StopReply,
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
    pub reply_poll_generation: u64,
    pub comment_expanded: bool,
    pub comment: text_editor::Content,
    pub comment_id: Option<String>,
    pub reply_to: Option<String>,
    pub attachments: Vec<String>,
    pub reply_menu_open: bool,
    pub reply_mode: Option<AgentSchedulerTaskReplyMode>,
    pub restart_turn_id: Option<String>,
    pub editor: Option<Field>,
    pub input: String,
    pub activity_expanded: bool,
    pub expanded_run: Option<String>,
}

impl State {
    pub fn reply_mode_allowed(&self, mode: AgentSchedulerTaskReplyMode) -> bool {
        let Some(detail) = self.data.as_ref().filter(|detail| detail.reply_agent.is_some()) else {
            return false;
        };
        match mode {
            AgentSchedulerTaskReplyMode::Start => matches!(
                detail.reply_session,
                AgentSchedulerTaskReplySession::Missing | AgentSchedulerTaskReplySession::Idle
            ),
            AgentSchedulerTaskReplyMode::Queue => {
                detail.reply_session == AgentSchedulerTaskReplySession::Busy
            }
            AgentSchedulerTaskReplyMode::Restart => detail.reply_active_turn_id.is_some(),
        }
    }

    pub fn has_active_replies(&self) -> bool {
        self.data.as_ref().is_some_and(|detail| {
            detail.collaboration.comments.iter().any(|comment| {
                comment.agent_turn.as_ref().is_some_and(|turn| {
                    matches!(
                        turn.status,
                        AgentSchedulerTaskAgentTurnStatus::Queued
                            | AgentSchedulerTaskAgentTurnStatus::Running
                    )
                })
            })
        })
    }

    pub fn can_post(&self) -> bool {
        let body = self.comment.text();
        self.data.is_some()
            && !self.write_pending()
            && (!body.trim().is_empty() || !self.attachments.is_empty())
            && body.trim().chars().count() <= 4000
            && self.reply_selection_current()
    }

    pub fn reply_selection_current(&self) -> bool {
        self.reply_mode.is_none_or(|mode| self.reply_mode_allowed(mode))
            && (self.reply_mode != Some(AgentSchedulerTaskReplyMode::Restart)
                || self.data.as_ref().and_then(|detail| detail.reply_active_turn_id.as_ref())
                    == self.restart_turn_id.as_ref())
    }

    pub fn write_pending(&self) -> bool {
        self.request.is_some_and(|(_, operation)| operation != Operation::PollReplies)
    }
}
