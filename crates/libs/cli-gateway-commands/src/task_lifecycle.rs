//! 任务生命周期 Workflow CRUD 命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleWorkflowUpsertBody;

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum TaskLifecycleCommands {
    List,
    Create {
        #[arg(long, value_parser = required)]
        input: String,
    },
    Get {
        #[arg(value_parser = required)]
        id: String,
    },
    Update {
        #[arg(value_parser = required)]
        id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Delete {
        #[arg(value_parser = required)]
        id: String,
    },
    Reset {
        #[arg(value_parser = required)]
        id: String,
    },
}

impl TaskLifecycleCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Create { .. } => Some("create task lifecycle workflow".to_string()),
            Self::Update { id, .. } => Some(format!("update task lifecycle workflow '{id}'")),
            Self::Delete { id } => Some(format!("delete task lifecycle workflow '{id}'")),
            Self::Reset { id } => Some(format!("reset task lifecycle workflow '{id}'")),
            Self::List | Self::Get { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: TaskLifecycleCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        TaskLifecycleCommands::List => {
            client.task_lifecycle_workflows_list().await.map(|v| json!(v))
        }
        TaskLifecycleCommands::Create { input } => {
            let body = parse(&input, stdin)?;
            client.task_lifecycle_workflows_create(&body).await.map(|v| json!(v))
        }
        TaskLifecycleCommands::Get { id } => {
            client.task_lifecycle_workflows_get(&id).await.map(|v| json!(v))
        }
        TaskLifecycleCommands::Update { id, input } => {
            let body = parse(&input, stdin)?;
            client.task_lifecycle_workflows_update(&id, &body).await.map(|v| json!(v))
        }
        TaskLifecycleCommands::Delete { id } => {
            client.task_lifecycle_workflows_delete(&id).await.map(|v| json!(v))
        }
        TaskLifecycleCommands::Reset { id } => {
            client.task_lifecycle_workflows_reset(&id).await.map(|v| json!(v))
        }
    }
    .map_err(anyhow::Error::msg)?;
    output::write_json(writer, &response, context.output_format())
}

fn parse(source: &str, stdin: &mut impl Read) -> Result<TaskLifecycleWorkflowUpsertBody> {
    let body: TaskLifecycleWorkflowUpsertBody =
        serde_json::from_value(read_json_input(source, stdin)?)
            .context("invalid task lifecycle workflow JSON input")?;
    body.workflow.validate().map_err(anyhow::Error::msg)?;
    Ok(body)
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
