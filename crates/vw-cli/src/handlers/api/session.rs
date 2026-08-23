//! Session 核心生命周期命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionPatchBody, GatewaySessionResetBody, GatewaySessionSummarizeBody,
    GatewaySessionTitleGenerateBody, GatewaySessionTodoPutBody,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum SessionCommands {
    /// 列出会话。
    List,
    /// 读取会话运行状态表。
    Status,
    /// 创建会话。
    Create { #[arg(long, value_parser = required)] input: String },
    /// 读取会话详情。
    Detail { #[arg(value_parser = required)] session_id: String },
    /// 更新会话。
    Update {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 删除会话。
    Delete { #[arg(value_parser = required)] session_id: String },
    /// 读取直接子会话。
    Children { #[arg(value_parser = required)] session_id: String },
    /// 读取会话待办。
    TodoGet { #[arg(value_parser = required)] session_id: String },
    /// 覆盖会话待办。
    TodoPut {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 分叉会话。
    Fork {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 重置会话。
    Reset {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取会话差异。
    Diff {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        message_id: Option<String>,
    },
    /// 触发会话摘要。
    Summarize {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 生成会话标题。
    Title {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
}

impl SessionCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Create { .. } => Some("create session".to_string()),
            Self::Update { session_id, .. } => Some(format!("update session '{session_id}'")),
            Self::Delete { session_id } => Some(format!("delete session '{session_id}'")),
            Self::TodoPut { session_id, .. } => {
                Some(format!("replace todos for session '{session_id}'"))
            }
            Self::Fork { session_id, .. } => Some(format!("fork session '{session_id}'")),
            Self::Reset { session_id, .. } => Some(format!("reset session '{session_id}'")),
            Self::Summarize { session_id, .. } => {
                Some(format!("summarize session '{session_id}'"))
            }
            Self::Title { session_id, .. } => {
                Some(format!("generate title for session '{session_id}'"))
            }
            Self::List
            | Self::Status
            | Self::Detail { .. }
            | Self::Children { .. }
            | Self::TodoGet { .. }
            | Self::Diff { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: SessionCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let directory = context.directory();
    let response = match command {
        SessionCommands::List => client.session_list::<Value>(directory).await,
        SessionCommands::Status => client.session_status::<Value>(directory).await,
        SessionCommands::Create { input } => {
            let directory = directory.context("api session create requires --directory")?;
            let body = parse::<GatewaySessionCreateBody>(&input, stdin, "create")?;
            client.session_create::<Value>(directory, &Some(body)).await
        }
        SessionCommands::Detail { session_id } => {
            client.session_get::<Value>(&session_id, directory).await
        }
        SessionCommands::Update { session_id, input } => {
            let body = parse::<GatewaySessionPatchBody>(&input, stdin, "update")?;
            client.session_update::<Value>(&session_id, directory, &body).await
        }
        SessionCommands::Delete { session_id } => {
            client.session_delete(&session_id, directory).await.map(|()| Value::Null)
        }
        SessionCommands::Children { session_id } => {
            client.session_children::<Value>(&session_id, directory).await
        }
        SessionCommands::TodoGet { session_id } => {
            client.session_todo_get(&session_id, directory).await.map(|value| json!(value))
        }
        SessionCommands::TodoPut { session_id, input } => {
            let body = parse::<GatewaySessionTodoPutBody>(&input, stdin, "todo update")?;
            client.session_todo_update(&session_id, directory, &body).await.map(|value| json!(value))
        }
        SessionCommands::Fork { session_id, input } => {
            let body = parse::<GatewaySessionForkBody>(&input, stdin, "fork")?;
            client.session_fork::<Value>(&session_id, directory, &Some(body)).await
        }
        SessionCommands::Reset { session_id, input } => {
            let body = parse::<GatewaySessionResetBody>(&input, stdin, "reset")?;
            client.session_reset::<Value>(&session_id, directory, &body).await
        }
        SessionCommands::Diff { session_id, message_id } => {
            client
                .session_diff::<Value>(
                    &session_id,
                    &GatewaySessionDiffQuery {
                        directory: directory.map(ToOwned::to_owned),
                        message_id,
                    },
                )
                .await
        }
        SessionCommands::Summarize { session_id, input } => {
            let body = parse::<GatewaySessionSummarizeBody>(&input, stdin, "summarize")?;
            client.session_summarize(&session_id, directory, &body).await.map(|value| json!(value))
        }
        SessionCommands::Title { session_id, input } => {
            let body = parse::<GatewaySessionTitleGenerateBody>(&input, stdin, "title")?;
            client.session_title_generate(&session_id, &body).await.map(|value| json!(value))
        }
    }
    .map_err(anyhow::Error::msg)?;
    output::write_json(writer, &response, context.output_format())
}

fn parse<T: DeserializeOwned>(source: &str, stdin: &mut impl Read, operation: &str) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid session {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
