//! 子代理输入、状态和协作消息；不持有执行器或全局状态。
use serde::{Deserialize, Serialize};
use serde_json::Value;
use config_agent::DelegateCapabilityPolicy;

/// 启动一次持久化子会话所需的输入。
#[derive(Debug, Clone)]
pub struct ChildAgentRunInput {
    /// 父会话 ID。
    pub parent_session_id: String,
    /// 子会话运行工作区。
    pub directory: String,
    /// 要使用的委托 agent key。
    pub agent_key: String,
    /// 父会话当前模型的完整 `provider/model` 引用。
    pub model: Option<String>,
    /// 本轮子任务提示词。
    pub prompt: String,
    /// 提示词之前的 Gateway 格式上下文消息。
    pub context: Vec<Value>,
    /// 委托 agent 注册给模型的工具选择。
    pub allowed_tools: Vec<String>,
    /// 委托 agent 配置的默认技能选择。
    pub default_allowed_skills: Vec<String>,
    /// 委托 agent 的技能可见性策略。
    pub skill_policy: DelegateCapabilityPolicy,
    /// 从父会话运行时上下文固化的可信用户技能选择。
    pub trusted_user_selected_skills: Vec<String>,
}

/// 子会话执行结束后的终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildAgentTerminalState {
    /// Agent stream 正常完成。
    Succeeded,
    /// Agent stream 返回错误。
    Failed,
    /// 父智能体取消了仍在运行的子任务。
    Cancelled,
}

/// 一次持久化子会话执行的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildAgentRunResult {
    /// 新创建且已持久化的子会话 ID。
    pub child_session_id: String,
    /// 子会话终态。
    pub terminal_state: ChildAgentTerminalState,
    /// 成功时供父会话消费的最终交接报告。
    pub output: Option<String>,
    /// 失败时的错误文本。
    pub error: Option<String>,
}

/// 子任务生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildAgentTaskState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ChildAgentTaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// 子任务的稳定状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChildAgentTaskSnapshot {
    pub parent_session_id: String,
    pub child_session_id: String,
    pub agent: String,
    pub task: String,
    pub turn: u64,
    pub state: ChildAgentTaskState,
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_ms: u64,
    pub updated_ms: u64,
}

/// 等待任意任务或全部任务进入终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitCondition {
    Any,
    All,
}

/// 一次事件驱动等待的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildAgentWaitOutcome {
    pub tasks: Vec<ChildAgentTaskSnapshot>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ForkTurns {
    #[default]
    All,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TeamMessageKind {
    Message,
    Handoff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TeamMessage {
    pub id: String,
    pub sender_session: String,
    pub recipient_session: String,
    pub turn: u64,
    pub kind: TeamMessageKind,
    pub payload: String,
    pub sequence: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TeamActivity {
    Message,
    Result,
    Cancelled,
    TimedOut,
}
