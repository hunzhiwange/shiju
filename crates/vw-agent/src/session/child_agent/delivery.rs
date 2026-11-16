//! Validate the live tenant before consuming coordinator data.
use super::ChildAgentCoordinator;
use crate::tools::ToolUseContext;
use serde_json::Value;
impl ChildAgentCoordinator {
    pub(crate) fn deliver_model_events(&self, context: &ToolUseContext, messages: &mut Vec<Value>) -> anyhow::Result<bool> {
        session_engine::child_agent_access::deliver_model_events(&self.0, context, messages)
    }
}
#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
