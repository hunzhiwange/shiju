//! Action-specific validation and adapters to the shared team runtime.
use super::*;
use crate::session::child_agent::coordinator::mailbox::{TeamActivity, TeamMessageKind};

impl AgentTool {
    pub(super) fn validate_action_fields(
        &self,
        args: &Value,
        action: AgentToolAction,
    ) -> anyhow::Result<()> {
        let fields = args
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("AgentTool arguments must be an object"))?;
        let allowed: &[&str] = match action {
            AgentToolAction::Launch | AgentToolAction::Run => {
                &["agent", "subagent_type", "prompt", "context", "fork_turns"]
            }
            AgentToolAction::SendMessage | AgentToolAction::FollowupTask => &["target", "message"],
            AgentToolAction::ListAgents => &[],
            AgentToolAction::Status | AgentToolAction::Stop => {
                &["child_session_id", "child_session_ids"]
            }
            AgentToolAction::Wait => {
                &["child_session_id", "child_session_ids", "condition", "timeout_ms"]
            }
        };
        for (key, value) in fields {
            if key != "action" && !allowed.contains(&key.as_str()) {
                anyhow::bail!("Unsupported parameter '{key}' for this action");
            }
            if value.is_null() {
                anyhow::bail!("Parameter '{key}' cannot be null");
            }
        }
        if fields.get("action").is_some_and(|v| !v.is_string()) {
            anyhow::bail!("action must be a string");
        }
        if matches!(action, AgentToolAction::SendMessage | AgentToolAction::FollowupTask) {
            Self::required_text(args, "target")?;
            Self::required_text(args, "message")?;
        }
        if matches!(action, AgentToolAction::Launch | AgentToolAction::Run) {
            for key in ["agent", "subagent_type", "prompt", "context"] {
                if fields.get(key).is_some_and(|v| !v.is_string()) {
                    anyhow::bail!("'{key}' must be a string");
                }
            }
        }
        if action == AgentToolAction::Wait {
            if args.get("condition").is_some_and(|v| !v.is_string()) {
                anyhow::bail!("condition must be a string");
            }
            self.wait_timeout_from_args(args)?;
            if args.get("condition").and_then(Value::as_str) != Some("activity") {
                self.wait_condition_from_args(args)?;
            } else if args.get("child_session_id").is_some()
                || args.get("child_session_ids").is_some()
            {
                anyhow::bail!(
                    "activity waits observe the current session mailbox, not child targets"
                );
            }
        }
        Ok(())
    }

    fn required_text<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("'{key}' must be a non-empty string"))
    }

    fn communication_context(&self) -> anyhow::Result<Arc<super::super::context::ToolUseContext>> {
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("Team operations require tool runtime context"))?;
        if context.session() != self.parent_session_id()? {
            anyhow::bail!("Team runtime session mismatch");
        }
        Ok(context)
    }

    pub(super) async fn handle_communication(
        &self,
        args: &Value,
        action: AgentToolAction,
    ) -> anyhow::Result<ToolResult> {
        let context = self.communication_context()?;
        let tasks = self.coordinator.team_snapshots(&context).await?;
        let body = match action {
            AgentToolAction::ListAgents => json!({"action":"list_agents","tasks":tasks}),
            AgentToolAction::SendMessage => {
                let turn = tasks
                    .iter()
                    .find(|t| t.child_session_id == context.session())
                    .map_or(0, |t| t.turn);
                let message = self
                    .coordinator
                    .send_team_message(
                        &context,
                        Self::required_text(args, "target")?,
                        turn,
                        TeamMessageKind::Message,
                        Self::required_text(args, "message")?.into(),
                    )
                    .await?;
                json!({"action":"send_message","message":message})
            }
            AgentToolAction::FollowupTask => {
                self.coordinator.followup_task(
                    context.session(),
                    Self::required_text(args, "target")?,
                    Self::required_text(args, "message")?.into(),
                )?;
                json!({"action":"followup_task","queued":true})
            }
            _ => anyhow::bail!("Unsupported communication action"),
        };
        Ok(ToolResult { success: true, output: serde_json::to_string(&body)?, error: None })
    }

    pub(super) async fn handle_activity(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let context = self.communication_context()?;
        let cancellation = context.abort_token().cloned().unwrap_or_default();
        let activity = self
            .coordinator
            .wait_team_activity(&context, self.wait_timeout_from_args(args)?, &cancellation)
            .await?;
        let event = match activity {
            TeamActivity::Message => "message",
            TeamActivity::Result => "result",
            TeamActivity::Cancelled => "cancelled",
            TeamActivity::TimedOut => "timed_out",
        };
        Ok(ToolResult {success:true, output:json!({"action":"wait","condition":"activity","activity":event,"timed_out":activity == TeamActivity::TimedOut}).to_string(), error:None})
    }
}
