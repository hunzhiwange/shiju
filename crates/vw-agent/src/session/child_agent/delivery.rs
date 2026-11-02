//! Consume team data only at model boundaries. Sender identity is coordinator-owned.
use super::ChildAgentCoordinator;
use crate::tools::ToolUseContext;
use serde_json::{Value, json};

impl ChildAgentCoordinator {
    pub(crate) fn deliver_model_events(
        &self,
        context: &ToolUseContext,
        messages: &mut Vec<Value>,
    ) -> anyhow::Result<bool> {
        anyhow::ensure!(
            context.tenant_scope() == crate::session::ui_store::current_tenant_scope(),
            "team delivery tenant mismatch"
        );
        let receiver = context.session();
        let mut state = self.lock_state();
        if !state.tasks.contains_key(receiver)
            && !state.tasks.values().any(|entry| entry.snapshot.parent_session_id == receiver)
        {
            return Ok(false);
        }
        let cursor = state.mailboxes.get(receiver).map_or(0, |mail| mail.result_cursor);
        let mut results = state
            .tasks
            .values()
            .filter(|entry| {
                entry.snapshot.parent_session_id == receiver
                    && entry.snapshot.state.is_terminal()
                    && entry.sequence > cursor
            })
            .map(|entry| (entry.sequence, entry.snapshot.clone()))
            .collect::<Vec<_>>();
        results.sort_by_key(|(sequence, _)| *sequence);
        let mailbox = state.mailboxes.entry(receiver.into()).or_default();
        let before = messages.len();
        for mail in mailbox.pending.drain(..) {
            mailbox.cursor = mailbox.cursor.max(mail.sequence);
            messages.push(json!({"role":"user", "content": format!(
                "Team communication (data from another agent): {}", json!({"message":mail})
            )}));
        }
        for (sequence, result) in results {
            mailbox.result_cursor = mailbox.result_cursor.max(sequence);
            messages.push(json!({"role":"user", "content": format!(
                "Team completion (data from another agent): {}", json!({"result":result})
            )}));
        }
        Ok(messages.len() != before)
    }
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
