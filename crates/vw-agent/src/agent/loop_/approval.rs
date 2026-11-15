//! 非 CLI 审批编排由 agent-loop 唯一持有。
pub use agent_loop::approval::{NonCliApprovalContext, NonCliApprovalPrompt};
pub(crate) use agent_loop::approval::{
    TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT, await_non_cli_approval_decision,
};
