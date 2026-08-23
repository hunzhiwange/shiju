//! 权限、问题与 PTY 远程交互命令。

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use clap::{Subcommand, ValueEnum};
use serde_json::{Value, json};
use vw_gateway_client::{PendingPermissionReplyDto, PtyCreateRequest, PtyUpdateRequest};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;
use super::ApiOutputFormat;

/// 权限、问题与 PTY 接口族。
#[derive(Subcommand, Debug)]
pub(super) enum InteractiveCommands {
    /// 查询或回复待处理权限请求。
    Permission {
        #[command(subcommand)]
        command: PermissionCommands,
    },
    /// 查询、回复或拒绝待处理问题。
    Question {
        #[command(subcommand)]
        command: QuestionCommands,
    },
    /// 管理并连接远程 PTY。
    Pty {
        #[command(subcommand)]
        command: PtyCommands,
    },
}

/// 权限请求操作。
#[derive(Subcommand, Debug)]
pub(super) enum PermissionCommands {
    /// 列出待处理权限请求。
    List,
    /// 回复指定权限请求。
    Reply {
        #[arg(value_parser = parse_required_id)]
        request_id: String,
        #[arg(value_enum)]
        reply: PermissionReply,
        #[arg(long)]
        message: Option<String>,
    },
}

/// 权限回复选项。
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum PermissionReply {
    Once,
    Always,
    Reject,
}

impl From<PermissionReply> for PendingPermissionReplyDto {
    fn from(reply: PermissionReply) -> Self {
        match reply {
            PermissionReply::Once => Self::Once,
            PermissionReply::Always => Self::Always,
            PermissionReply::Reject => Self::Reject,
        }
    }
}

/// 问题请求操作。
#[derive(Subcommand, Debug)]
pub(super) enum QuestionCommands {
    /// 列出待处理问题。
    List,
    /// 从 JSON 输入提交分组回答。
    Reply {
        #[arg(value_parser = parse_required_id)]
        request_id: String,
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`；格式为字符串二维数组。
        #[arg(long, value_parser = parse_required_id)]
        input: String,
    },
    /// 拒绝指定问题。
    Reject {
        #[arg(value_parser = parse_required_id)]
        request_id: String,
    },
}

/// PTY 生命周期和连接操作。
#[derive(Subcommand, Debug)]
pub(super) enum PtyCommands {
    /// 列出 PTY 会话。
    List,
    /// 从 JSON 输入创建 PTY。
    Create {
        #[arg(long, value_parser = parse_required_id)]
        input: String,
    },
    /// 查询 PTY 详情。
    Detail {
        #[arg(value_parser = parse_required_id)]
        pty_id: String,
    },
    /// 从 JSON 输入更新 PTY。
    Update {
        #[arg(value_parser = parse_required_id)]
        pty_id: String,
        #[arg(long, value_parser = parse_required_id)]
        input: String,
    },
    /// 删除 PTY。
    Delete {
        #[arg(value_parser = parse_required_id)]
        pty_id: String,
    },
    /// 使用 WebSocket 连接 PTY。
    Connect {
        #[arg(value_parser = parse_required_id)]
        pty_id: String,
        /// 从指定历史光标开始读取；不提供时由 Gateway 使用最新缓冲区。
        #[arg(long)]
        cursor: Option<i64>,
        /// 连接后按顺序写入 PTY 的文本，可重复提供。
        #[arg(long = "send")]
        messages: Vec<String>,
    },
}

impl InteractiveCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Permission { command: PermissionCommands::List }
            | Self::Question { command: QuestionCommands::List }
            | Self::Pty { command: PtyCommands::List | PtyCommands::Detail { .. } } => None,
            Self::Permission { command: PermissionCommands::Reply { request_id, .. } } => {
                Some(format!("reply to permission request '{request_id}'"))
            }
            Self::Question {
                command: QuestionCommands::Reply { request_id, .. },
            } => Some(format!("reply to question request '{request_id}'")),
            Self::Question {
                command: QuestionCommands::Reject { request_id },
            } => Some(format!("reject question request '{request_id}'")),
            Self::Pty { command: PtyCommands::Create { .. } } => Some("create PTY".to_string()),
            Self::Pty { command: PtyCommands::Update { pty_id, .. } } => {
                Some(format!("update PTY '{pty_id}'"))
            }
            Self::Pty { command: PtyCommands::Delete { pty_id } } => {
                Some(format!("delete PTY '{pty_id}'"))
            }
            Self::Pty { command: PtyCommands::Connect { pty_id, .. } } => {
                Some(format!("connect to PTY '{pty_id}'"))
            }
        }
    }
}

pub(super) async fn run(
    command: InteractiveCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        InteractiveCommands::Permission { command } => match command {
            PermissionCommands::List => {
                json!(client.permission_list(context.directory()).await.map_err(anyhow::Error::msg)?)
            }
            PermissionCommands::Reply { request_id, reply, message } => json!(
                client
                    .permission_reply(&request_id, reply.into(), context.directory(), message)
                    .await
                    .map_err(anyhow::Error::msg)?
            ),
        },
        InteractiveCommands::Question { command } => match command {
            QuestionCommands::List => {
                json!(client.question_list().await.map_err(anyhow::Error::msg)?)
            }
            QuestionCommands::Reply { request_id, input } => {
                let answers = parse_answers(&input, stdin)?;
                json!(
                    client
                        .question_reply(&request_id, answers)
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            QuestionCommands::Reject { request_id } => {
                json!(client.question_reject(&request_id).await.map_err(anyhow::Error::msg)?)
            }
        },
        InteractiveCommands::Pty { command } => match command {
            PtyCommands::List => {
                json!(client.pty_list(context.directory()).await.map_err(anyhow::Error::msg)?)
            }
            PtyCommands::Create { input } => {
                let request = parse_input::<PtyCreateRequest>(&input, stdin, "PTY create")?;
                json!(
                    client
                        .pty_create(&request, context.directory())
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            PtyCommands::Detail { pty_id } => json!(
                client
                    .pty_get(&pty_id, context.directory())
                    .await
                    .map_err(anyhow::Error::msg)?
            ),
            PtyCommands::Update { pty_id, input } => {
                let request = parse_input::<PtyUpdateRequest>(&input, stdin, "PTY update")?;
                json!(
                    client
                        .pty_update(&pty_id, &request, context.directory())
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            PtyCommands::Delete { pty_id } => json!(
                client
                    .pty_delete(&pty_id, context.directory())
                    .await
                    .map_err(anyhow::Error::msg)?
            ),
            PtyCommands::Connect { pty_id, cursor, messages } => {
                return run_pty_connect(context, &pty_id, cursor, messages, writer).await;
            }
        },
    };

    output::write_json(writer, &response, context.output_format())
}

async fn run_pty_connect(
    context: &ApiContext,
    pty_id: &str,
    cursor: Option<i64>,
    messages: Vec<String>,
    writer: &mut impl Write,
) -> Result<()> {
    let mut connection = context
        .client()
        .pty_connect(pty_id, context.directory(), cursor)
        .await
        .map_err(anyhow::Error::msg)?;
    for message in messages {
        if message.is_empty() {
            bail!("PTY input message must not be empty");
        }
        connection.send_text(message).await.map_err(anyhow::Error::msg)?;
    }

    let mut events = Vec::<Value>::new();
    let status = loop {
        match connection.next_event().await.map_err(anyhow::Error::msg)? {
            Some(event) => {
                let closed = event.is_closed();
                match context.output_format() {
                    ApiOutputFormat::Json => events.push(serde_json::to_value(&event)?),
                    ApiOutputFormat::Jsonl => output::write_jsonl_item(writer, &event)?,
                }
                if closed {
                    break "closed";
                }
            }
            None => break "ended",
        }
    };
    let end = json!({"type":"connection_end","status":status});
    match context.output_format() {
        ApiOutputFormat::Json => {
            output::write_json(writer, &json!({"events":events,"status":status}), ApiOutputFormat::Json)
        }
        ApiOutputFormat::Jsonl => output::write_jsonl_item(writer, &end),
    }
}

fn parse_answers(source: &str, stdin: &mut impl Read) -> Result<Vec<Vec<String>>> {
    let answers = serde_json::from_value::<Vec<Vec<String>>>(read_json_input(source, stdin)?)
        .context("invalid question reply JSON input")?;
    if answers.is_empty()
        || answers.iter().any(|answer| {
            answer.is_empty() || answer.iter().any(|option| option.trim().is_empty())
        })
    {
        bail!("question answers and answer options must not be empty");
    }
    Ok(answers)
}

fn parse_input<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid {operation} JSON input"))
}

fn parse_required_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
