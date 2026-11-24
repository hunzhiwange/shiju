//! Project-scoped collaboration through the workbench gateway.

use std::io::{Read, Write};
use std::time::Duration;

use anyhow::Result;
use clap::{Args, Subcommand};
use cli_api_core::{ApiOptions, context::ApiContext, ensure_confirmed, output::write_json};
use serde::Serialize;
use serde_json::Value;
use tokio::time::{Instant, timeout_at};
use vw_api_transport::UserId;
use vw_gateway_client::cli_workbench::{
    CHILD_CONTEXT_ENV, CliWorkbenchChildContext, CliWorkbenchError, CliWorkbenchErrorCode,
    CliWorkbenchMessage, CliWorkbenchMessageState, CliWorkbenchTarget, SendCliWorkbenchMessage,
    StartCliWorkbenchSession,
};
use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};

const MAX_TEXT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Args)]
pub struct WorkbenchArgs {
    #[command(flatten)]
    options: ApiOptions,

    /// Registered project ID. Required outside a workbench child.
    #[arg(long, global = true)]
    project: Option<String>,

    /// Explicit gateway user ID for calls outside a workbench child.
    #[arg(long, global = true)]
    user: Option<String>,

    #[command(subcommand)]
    command: WorkbenchCommand,
}

#[derive(Debug, Subcommand)]
enum WorkbenchCommand {
    /// List sessions in the current project.
    List,
    /// Start a workbench session. Requires --yes.
    Start {
        #[command(subcommand)]
        target: StartTarget,
    },
    /// Send UTF-8 text from stdin to a session. Requires --yes.
    Send {
        session_id: String,
        /// Unique request ID chosen before sending. Reuse it when checking this delivery.
        #[arg(long)]
        request_id: String,
    },
    /// Get the latest state of a specific delivery.
    Status(Delivery),
    /// Wait for a result or an action that requires the user.
    Wait {
        #[command(flatten)]
        delivery: Delivery,

        #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..=3600))]
        timeout_secs: u64,
    },
    /// Read the output and state of a specific delivery.
    Read(Delivery),
}

#[derive(Debug, Subcommand)]
enum StartTarget {
    /// Start an auxiliary session using a configured ACP agent profile.
    Acp { agent: String },
    /// Start a supported native CLI by application ID.
    NativeCli { app_id: String },
    /// Start a shell.
    Shell,
}

impl From<StartTarget> for CliWorkbenchTarget {
    fn from(target: StartTarget) -> Self {
        match target {
            StartTarget::Acp { agent } => Self::Acp { agent },
            StartTarget::NativeCli { app_id } => Self::NativeCli { app_id },
            StartTarget::Shell => Self::Shell,
        }
    }
}

#[derive(Debug, Args)]
struct Delivery {
    session_id: String,
    #[arg(long)]
    request_id: String,
}

#[derive(Serialize)]
struct Failure {
    error: CliWorkbenchError,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Box<CliWorkbenchMessage>>,
}

impl Failure {
    fn new(code: CliWorkbenchErrorCode, message: impl Into<String>) -> Self {
        CliWorkbenchError { code, message: message.into() }.into()
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(CliWorkbenchErrorCode::InvalidRequest, message)
    }
}

impl From<CliWorkbenchError> for Failure {
    fn from(error: CliWorkbenchError) -> Self {
        Self { error, message: None }
    }
}

pub async fn run(args: WorkbenchArgs) -> Result<()> {
    run_with_io(
        args,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        std::env::var_os(CHILD_CONTEXT_ENV),
    )
    .await
}

async fn run_with_io(
    args: WorkbenchArgs,
    stdin: &mut impl Read,
    writer: &mut impl Write,
    child_context: Option<std::ffi::OsString>,
) -> Result<()> {
    let format = args.options.output();
    match execute(args, stdin, child_context).await {
        Ok(result) => {
            write_json(writer, &serde_json::json!({ "ok": true, "result": result }), format)
        }
        Err(failure) => {
            #[derive(Serialize)]
            struct ErrorOutput<'a> {
                ok: bool,
                #[serde(flatten)]
                failure: &'a Failure,
            }
            write_json(writer, &ErrorOutput { ok: false, failure: &failure }, format)?;
            anyhow::bail!("{}", failure.error.message);
        }
    }
}

fn connection(
    args: &WorkbenchArgs,
    child_context: Option<std::ffi::OsString>,
) -> Result<(GatewayClient, String), Failure> {
    if let Some(context) = child_context {
        let parse_error = || Failure::invalid("invalid workbench child context");
        let context = context.into_string().map_err(|_| parse_error())?;
        let context: CliWorkbenchChildContext =
            serde_json::from_str(&context).map_err(|_| parse_error())?;
        if context.port == 0 || context.project_id.trim().is_empty() || context.token.is_empty() {
            return Err(parse_error());
        }
        if args.user.is_some() {
            return Err(Failure::invalid("--user is only available outside a workbench child"));
        }
        if args.project.as_ref().is_some_and(|id| id != &context.project_id) {
            return Err(Failure::new(
                CliWorkbenchErrorCode::Unauthorized,
                "workbench child is bound to its current project",
            ));
        }
        let endpoint = GatewayEndpoint::new("127.0.0.1", context.port)
            .with_auth(GatewayAuth { skey: Some(context.token) });
        let client = GatewayClient::new(endpoint)
            .map_err(|_| Failure::invalid("invalid workbench endpoint"))?;
        return Ok((client, context.project_id));
    }

    let user = args.user.as_deref().ok_or_else(|| Failure::invalid("--user is required"))?;
    let user = UserId::new(user).map_err(|e| Failure::invalid(e.to_string()))?;
    let project = args
        .project
        .as_ref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| Failure::invalid("--project is required"))?;
    let context =
        ApiContext::for_user(&args.options, user).map_err(|e| Failure::invalid(e.to_string()))?;
    Ok((context.client().clone(), project.clone()))
}

async fn execute(
    args: WorkbenchArgs,
    stdin: &mut impl Read,
    child_context: Option<std::ffi::OsString>,
) -> Result<Value, Failure> {
    let (client, project) = connection(&args, child_context)?;
    let operation = match &args.command {
        WorkbenchCommand::Start { .. } => Some("workbench start"),
        WorkbenchCommand::Send { .. } => Some("workbench send"),
        _ => None,
    };
    if let Some(operation) = operation {
        ensure_confirmed(args.options.yes(), operation)
            .map_err(|e| Failure::invalid(e.to_string()))?;
    }

    match args.command {
        WorkbenchCommand::List => encode(&client.cli_workbench_list(&project).await?),
        WorkbenchCommand::Start { target } => {
            let request = StartCliWorkbenchSession {
                target: target.into(),
                directory: args.options.directory().map(str::to_owned),
            };
            encode(&client.cli_workbench_start(&project, &request).await?)
        }
        WorkbenchCommand::Send { session_id, request_id } => {
            let mut bytes = Vec::new();
            stdin
                .take(MAX_TEXT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| Failure::invalid("could not read message from stdin"))?;
            if bytes.len() as u64 > MAX_TEXT_BYTES {
                return Err(Failure::invalid("message exceeds 65536 UTF-8 bytes"));
            }
            let text =
                String::from_utf8(bytes).map_err(|_| Failure::invalid("message must use UTF-8"))?;
            if text.trim().is_empty() {
                return Err(Failure::invalid("message must not be blank"));
            }
            let request = SendCliWorkbenchMessage { request_id, text };
            encode(&client.cli_workbench_send(&project, &session_id, &request).await?)
        }
        WorkbenchCommand::Status(delivery) | WorkbenchCommand::Read(delivery) => {
            encode(&receive(&client, &project, &delivery).await?)
        }
        WorkbenchCommand::Wait { delivery, timeout_secs } => {
            encode(&wait(&client, &project, &delivery, Duration::from_secs(timeout_secs)).await?)
        }
    }
}

async fn receive(
    client: &GatewayClient,
    project: &str,
    delivery: &Delivery,
) -> Result<CliWorkbenchMessage, CliWorkbenchError> {
    client.cli_workbench_receive(project, &delivery.session_id, &delivery.request_id).await
}

async fn wait(
    client: &GatewayClient,
    project: &str,
    delivery: &Delivery,
    timeout: Duration,
) -> Result<CliWorkbenchMessage, Failure> {
    let deadline = Instant::now() + timeout;
    let mut message = timeout_at(deadline, receive(client, project, delivery))
        .await
        .map_err(|_| wait_timeout(None))??;
    while !message.state.is_terminal()
        && !matches!(
            message.state,
            CliWorkbenchMessageState::PendingSubmission
                | CliWorkbenchMessageState::WaitingForApproval
        )
    {
        message = match timeout_at(
            deadline,
            client.cli_workbench_subscribe(
                project,
                &delivery.session_id,
                &delivery.request_id,
                message.revision,
            ),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => return Err(wait_timeout(Some(message))),
        };
    }
    Ok(message)
}

fn wait_timeout(message: Option<CliWorkbenchMessage>) -> Failure {
    let mut failure = Failure::new(
        CliWorkbenchErrorCode::TimedOut,
        "wait timed out; use status or read with the same session and request IDs",
    );
    failure.message = message.map(Box::new);
    failure
}

fn encode(value: &impl Serialize) -> Result<Value, Failure> {
    serde_json::to_value(value).map_err(|_| {
        Failure::new(CliWorkbenchErrorCode::GatewayFailure, "could not encode workbench response")
    })
}

#[cfg(test)]
#[path = "cli_workbench_tests.rs"]
mod tests;
