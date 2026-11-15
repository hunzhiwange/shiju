//! Session 消息内容、UI 快照和状态命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use vw_gateway_client::GatewaySessionMessageListQuery;
use vw_gateway_client::vw_api_types::session::GatewaySessionScopeBody;
use vw_shared::session::ui_types::ChatSession;

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum SessionContentCommands {
    Messages {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    Message {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(value_parser = required)]
        message_id: String,
    },
    PartUpdate {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(value_parser = required)]
        message_id: String,
        #[arg(value_parser = required)]
        part_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    PartDelete {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(value_parser = required)]
        message_id: String,
        #[arg(value_parser = required)]
        part_id: String,
    },
    UiPreviews,
    ArchivedGet,
    ArchivedPut {
        #[arg(long, value_parser = required)]
        input: String,
    },
    ScopeGet,
    ScopePut {
        #[arg(long, value_parser = required)]
        input: String,
    },
    UiGet {
        #[arg(value_parser = required)]
        session_id: String,
    },
    UiSave {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    Preview {
        #[arg(value_parser = required)]
        session_id: String,
    },
    Path {
        #[arg(value_parser = required)]
        session_id: String,
    },
    Any {
        #[arg(value_parser = required)]
        session_id: String,
    },
    Raw {
        #[arg(value_parser = required)]
        session_id: String,
        #[arg(value_parser = required)]
        turn_id: String,
        step_index: u32,
    },
}

impl SessionContentCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::PartUpdate { session_id, message_id, part_id, .. } => Some(format!(
                "update session '{session_id}' message '{message_id}' part '{part_id}'"
            )),
            Self::PartDelete { session_id, message_id, part_id } => Some(format!(
                "delete session '{session_id}' message '{message_id}' part '{part_id}'"
            )),
            Self::ArchivedPut { .. } => Some("replace archived session ids".to_string()),
            Self::ScopePut { .. } => Some("update global session scope".to_string()),
            Self::UiSave { session_id, .. } => {
                Some(format!("save UI snapshot for session '{session_id}'"))
            }
            Self::Messages { .. }
            | Self::Message { .. }
            | Self::UiPreviews
            | Self::ArchivedGet
            | Self::ScopeGet
            | Self::UiGet { .. }
            | Self::Preview { .. }
            | Self::Path { .. }
            | Self::Any { .. }
            | Self::Raw { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: SessionContentCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let directory = context.directory();
    let response = match command {
        SessionContentCommands::Messages { session_id, limit } => {
            client
                .session_messages_query::<Value>(
                    &session_id,
                    &GatewaySessionMessageListQuery {
                        directory: directory.map(ToOwned::to_owned),
                        limit,
                        cursor: None,
                    },
                )
                .await
        }
        SessionContentCommands::Message { session_id, message_id } => {
            client.session_message_get::<Value>(&session_id, &message_id, directory).await
        }
        SessionContentCommands::PartUpdate { session_id, message_id, part_id, input } => {
            let body = read_json_input(&input, stdin)?;
            client
                .session_message_part_update::<_, Value>(
                    &session_id,
                    &message_id,
                    &part_id,
                    directory,
                    &body,
                )
                .await
        }
        SessionContentCommands::PartDelete { session_id, message_id, part_id } => client
            .session_message_part_delete(&session_id, &message_id, &part_id, directory)
            .await
            .map(|value| json!(value)),
        SessionContentCommands::UiPreviews => {
            client.session_ui_previews(directory).await.map(|value| json!(value))
        }
        SessionContentCommands::ArchivedGet => {
            client.session_archived_get(directory).await.map(|value| json!(value))
        }
        SessionContentCommands::ArchivedPut { input } => {
            let body = parse::<Vec<String>>(&input, stdin, "archived")?;
            client.session_archived_put(directory, &body).await.map(|value| json!(value))
        }
        SessionContentCommands::ScopeGet => {
            client.session_scope_get(directory, None).await.map(|value| json!(value))
        }
        SessionContentCommands::ScopePut { input } => {
            let body = parse::<GatewaySessionScopeBody>(&input, stdin, "scope")?;
            client.session_scope_put(body.scope.as_deref()).await.map(|value| json!(value))
        }
        SessionContentCommands::UiGet { session_id } => {
            client.session_ui_get(&session_id, directory).await.map(|value| json!(value))
        }
        SessionContentCommands::UiSave { session_id, input } => {
            let body = parse::<ChatSession>(&input, stdin, "UI snapshot")?;
            if body.id != session_id {
                anyhow::bail!("session UI body id does not match session id");
            }
            client.session_ui_save(&session_id, directory, &body).await.map(|value| json!(value))
        }
        SessionContentCommands::Preview { session_id } => {
            client.session_preview_meta_get(&session_id, directory).await.map(|value| json!(value))
        }
        SessionContentCommands::Path { session_id } => {
            client.session_path_get(&session_id, directory).await.map(|value| json!(value))
        }
        SessionContentCommands::Any { session_id } => {
            client.session_ui_get_any(&session_id).await.map(|value| json!(value))
        }
        SessionContentCommands::Raw { session_id, turn_id, step_index } => client
            .session_step_llm_raw_get(&session_id, &turn_id, step_index, directory)
            .await
            .map(|value| json!(value)),
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
