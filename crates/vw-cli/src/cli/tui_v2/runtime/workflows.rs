use std::collections::BTreeMap;

use vw_gateway_client::{GatewayWorkflowResponseMode, GatewayWorkflowStreamRequest};

use super::super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandSource, CommandSpec,
};
use super::super::model::workflow_panel::WorkflowPanelItem;
use super::gateway::{GatewayUiRuntime, block_on_gateway};

pub(crate) trait WorkflowRuntime {
    fn list_workflows(&mut self) -> Result<Vec<WorkflowPanelItem>, String>;
    fn run_workflow(&mut self, uuid: &str, query: &str) -> Result<String, String>;
}

pub(crate) fn workflow_command_specs(items: &[WorkflowPanelItem]) -> Vec<CommandSpec> {
    items
        .iter()
        .map(|item| {
            let label = normalize_workflow_name(item.name.as_str());
            let suffix = normalize_workflow_name(item.uuid.as_str());
            CommandSpec {
                name: format!("workflow:{label}--{suffix}"),
                aliases: Vec::new(),
                summary: item.description.clone(),
                takes_argument: true,
                argument_hint: Some("<input>".to_string()),
                source: CommandSource::Workflow,
                availability: CommandAvailability::Available,
                execution_policy: CommandExecutionPolicy::Queued,
                action: CommandAction::Gateway(format!("workflow:{}", item.uuid)),
            }
        })
        .collect()
}

fn normalize_workflow_name(value: &str) -> String {
    let normalized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = normalized.trim_matches('-');
    if normalized.is_empty() { "unnamed".to_string() } else { normalized.to_string() }
}

impl WorkflowRuntime for GatewayUiRuntime {
    fn list_workflows(&mut self) -> Result<Vec<WorkflowPanelItem>, String> {
        block_on_gateway(self.client().workflow_applications_list()).map(|items| {
            items
                .into_iter()
                .map(|item| WorkflowPanelItem {
                    uuid: item.uuid,
                    name: item.name,
                    description: item.description,
                })
                .collect()
        })
    }

    fn run_workflow(&mut self, uuid: &str, query: &str) -> Result<String, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("workflow input is required".to_string());
        }
        let user = self.session_id().unwrap_or("tui").to_string();
        let request = GatewayWorkflowStreamRequest {
            query: query.to_string(),
            inputs: BTreeMap::new(),
            response_mode: GatewayWorkflowResponseMode::Streaming,
            user,
            conversation_id: None,
        };
        block_on_gateway(self.client().run_workflow_application(uuid, &request))
            .map(|outcome| outcome.summary)
    }
}
