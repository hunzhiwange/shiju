//! Validate the live tenant before consuming coordinator data.
use super::ChildAgentCoordinator;
use crate::tools::ToolUseContext;
use serde_json::Value;
impl ChildAgentCoordinator {
    pub(crate) fn deliver_model_events(&self, context: &ToolUseContext, messages: &mut Vec<Value>) -> anyhow::Result<bool> {
        anyhow::ensure!(context.tenant_scope() == crate::session::ui_store::current_tenant_scope(), "team delivery tenant mismatch");
        self.0.deliver_model_events(context.session(), messages)
    }
}
#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
