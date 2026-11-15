//! Workflow 应用生命周期、恢复与聊天流命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::vw_api_types::workflow::{WorkflowRecordUpsertBody, WorkflowResumeRequest};
use vw_gateway_client::{
    GatewayWorkflowDefaultStreamRequest, GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum WorkflowCommands {
    /// 恢复暂停的 Workflow 运行。
    Resume {
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 列出保存的 Workflow 应用。
    List,
    /// 从 JSON 输入创建 Workflow 应用。
    Create {
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取 Workflow 应用详情。
    Detail {
        #[arg(value_parser = required)]
        uuid: String,
    },
    /// 从 JSON 输入更新 Workflow 应用。
    Update {
        #[arg(value_parser = required)]
        uuid: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 删除 Workflow 应用。
    Delete {
        #[arg(value_parser = required)]
        uuid: String,
    },
    /// 通过默认入口流式运行保存应用或内联 Workflow。
    Chat {
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 通过 UUID 路径流式运行保存的 Workflow 应用。
    ChatApplication {
        #[arg(value_parser = required)]
        uuid: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
}

impl WorkflowCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Resume { .. } => Some("resume workflow run".to_string()),
            Self::Create { .. } => Some("create workflow application".to_string()),
            Self::Update { uuid, .. } => Some(format!("update workflow application '{uuid}'")),
            Self::Delete { uuid } => Some(format!("delete workflow application '{uuid}'")),
            Self::Chat { .. } => Some("run workflow chat stream".to_string()),
            Self::ChatApplication { uuid, .. } => {
                Some(format!("run workflow application '{uuid}' chat stream"))
            }
            Self::List | Self::Detail { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: WorkflowCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        WorkflowCommands::Resume { input } => {
            let request = parse::<WorkflowResumeRequest>(&input, stdin, "resume")?;
            json!(client.workflow_resume(&request).await.map_err(anyhow::Error::msg)?)
        }
        WorkflowCommands::List => {
            json!(client.workflow_applications_list().await.map_err(anyhow::Error::msg)?)
        }
        WorkflowCommands::Create { input } => {
            let request = parse::<WorkflowRecordUpsertBody>(&input, stdin, "create")?;
            json!(client.workflow_application_create(&request).await.map_err(anyhow::Error::msg)?)
        }
        WorkflowCommands::Detail { uuid } => {
            json!(client.workflow_application_get(&uuid).await.map_err(anyhow::Error::msg)?)
        }
        WorkflowCommands::Update { uuid, input } => {
            let request = parse::<WorkflowRecordUpsertBody>(&input, stdin, "update")?;
            json!(
                client
                    .workflow_application_update(&uuid, &request)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        WorkflowCommands::Delete { uuid } => {
            json!(client.workflow_application_delete(&uuid).await.map_err(anyhow::Error::msg)?)
        }
        WorkflowCommands::Chat { input } => {
            let request = parse::<GatewayWorkflowDefaultStreamRequest>(&input, stdin, "chat")?;
            return run_default_stream(context, &request, writer).await;
        }
        WorkflowCommands::ChatApplication { uuid, input } => {
            let request = parse::<GatewayWorkflowStreamRequest>(&input, stdin, "application chat")?;
            return run_application_stream(context, &uuid, &request, writer).await;
        }
    };
    output::write_json(writer, &response, context.output_format())
}

async fn run_default_stream(
    context: &ApiContext,
    request: &GatewayWorkflowDefaultStreamRequest,
    writer: &mut impl Write,
) -> Result<()> {
    let mut sink = WorkflowStreamSink::new(writer);
    let result = context
        .client()
        .stream_default_workflow_application(request, |event| sink.write(event))
        .await;
    sink.finish(result)
}

async fn run_application_stream(
    context: &ApiContext,
    uuid: &str,
    request: &GatewayWorkflowStreamRequest,
    writer: &mut impl Write,
) -> Result<()> {
    let mut sink = WorkflowStreamSink::new(writer);
    let result = context
        .client()
        .stream_workflow_application(uuid, request, |event| sink.write(event))
        .await;
    sink.finish(result)
}

struct WorkflowStreamSink<'a, W> {
    writer: &'a mut W,
    write_error: Option<anyhow::Error>,
    stream_error: Option<String>,
}

impl<'a, W: Write> WorkflowStreamSink<'a, W> {
    fn new(writer: &'a mut W) -> Self {
        Self { writer, write_error: None, stream_error: None }
    }

    fn write(&mut self, event: GatewayWorkflowStreamEvent) -> bool {
        match &event {
            GatewayWorkflowStreamEvent::Error(error) => {
                self.stream_error = Some(format!("{}: {}", error.code, error.message));
            }
            GatewayWorkflowStreamEvent::WorkflowFinished { data, .. } => {
                if let Some(status) = data.get("status").and_then(serde_json::Value::as_str)
                    && matches!(status, "failed" | "error")
                {
                    self.stream_error = Some(format!("workflow finished with status {status}"));
                }
            }
            _ => {}
        }
        if let Err(error) = output::write_jsonl_item(self.writer, &event) {
            self.write_error = Some(error);
            return false;
        }
        true
    }

    fn finish(self, result: Result<(), String>) -> Result<()> {
        result.map_err(anyhow::Error::msg)?;
        if let Some(error) = self.write_error {
            return Err(error);
        }
        if let Some(error) = self.stream_error {
            return Err(anyhow::Error::msg(error));
        }
        Ok(())
    }
}

fn parse<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid workflow {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
