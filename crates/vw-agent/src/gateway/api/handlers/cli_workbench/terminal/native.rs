use std::{io, sync::Arc};

use vw_agent_apps::apps::launch::workbench_pty::{PtySize, WorkbenchPty};
use vw_api_types::cli_workbench::{
    CLI_WORKBENCH_MAX_TERMINAL_INPUT, CliWorkbenchTerminalInput, CliWorkbenchTerminalOutput,
};

use super::*;

pub(in super::super) struct Pty {
    process: WorkbenchPty,
    input: tokio::sync::Mutex<()>,
}

impl Pty {
    pub(in super::super) fn exit_status(&self) -> io::Result<Option<std::process::ExitStatus>> {
        self.process.exit_status()
    }
}

pub(in super::super) async fn start(
    scope: ProjectScope,
    project_id: String,
    directory: String,
    id: String,
    target: CliWorkbenchTarget,
    authority: Authority,
) -> Result<(StatusCode, Json<CliWorkbenchSession>), WorkbenchError> {
    let child_access = if matches!(&target, CliWorkbenchTarget::NativeCli { .. }) {
        Some(child_access::ChildAccess::start(&scope, &id).await.map_err(|_| unavailable())?)
    } else {
        None
    };
    let context = child_access.as_ref().map(|access| access.context().to_owned());
    let (launch_scope, launch_target, cwd) = (scope.clone(), target.clone(), directory.clone());
    let process = tokio::task::spawn_blocking(move || {
        let mut command = gateway_agent_apps::launch::workbench_command(
            launch_scope.tenant_scope(),
            &launch_target,
            &cwd,
        )
        .map_err(|status| match status {
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
                error(status, ErrorCode::Unauthorized, "native launch is not authorized")
            }
            StatusCode::UNPROCESSABLE_ENTITY => error(
                status,
                ErrorCode::InvalidRequest,
                "native launch directory or installation is invalid",
            ),
            _ => unavailable(),
        })?;
        if let Some(context) = context {
            command.env(vw_api_types::cli_workbench::CHILD_CONTEXT_ENV, context);
        }
        WorkbenchPty::spawn(command, &agent_paths::paths().state, PtySize { rows: 24, cols: 80 })
            .map_err(|_| unavailable())
    })
    .await
    .map_err(|_| unavailable())??;
    let info = CliWorkbenchSession {
        id,
        project_id,
        directory,
        capabilities: capabilities(&target),
        target,
        state: ProcessState::Running,
    };
    insert_session(
        Session {
            scope,
            info,
            turn: None,
            terminal: Some(Arc::new(Pty { process, input: Default::default() })),
            child_access,
        },
        Some(&authority),
    )
}

pub(in super::super) fn stop(session: &mut Session) -> CliWorkbenchSession {
    session.refresh_process_state();
    if session.info.state == ProcessState::Running
        && let Some(pty) = session.terminal.clone()
    {
        session.info.state = ProcessState::Stopping;
        session.revoke_child_access();
        tokio::task::spawn_blocking(move || {
            let _ = pty.process.stop();
        });
    }
    session.info.clone()
}

pub(super) async fn shutdown() {
    let terminals = {
        let mut workbench = WORKBENCH.lock();
        let mut terminals = Vec::new();
        let mut removed = std::collections::HashSet::new();
        workbench.sessions.retain(|id, session| {
            if let Some(pty) = session.terminal.clone() {
                terminals.push(pty);
                removed.insert(id.clone());
                false
            } else {
                true
            }
        });
        workbench.deliveries.retain(|_, delivery| {
            !removed.contains(&delivery.updates.borrow().id.target_session_id)
        });
        terminals
    };
    if !terminals.is_empty() {
        let _ = tokio::task::spawn_blocking(move || {
            for pty in terminals {
                let _ = pty.process.stop();
            }
        })
        .await;
    }
}

fn terminal(
    tenant: TenantScope,
    project: String,
    session: &str,
) -> Result<Arc<Pty>, WorkbenchError> {
    let scope = project_scope(tenant, project)?;
    WORKBENCH.lock().session_mut(&scope, session)?.terminal.clone().ok_or_else(not_found)
}

#[derive(Deserialize)]
pub(super) struct OutputQuery {
    #[serde(default)]
    cursor: u64,
}

pub(super) async fn output(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session)): Path<(String, String)>,
    authority: Authority,
    Query(query): Query<OutputQuery>,
) -> Result<Json<CliWorkbenchTerminalOutput>, WorkbenchError> {
    authority.require()?;
    let pty = terminal(tenant, project, &session)?;
    let mut attachment = pty.process.attach(query.cursor).map_err(io_error)?;
    let output = pty.process.read(&mut attachment, Duration::ZERO).map_err(io_error)?;
    Ok(Json(CliWorkbenchTerminalOutput {
        bytes: output.bytes,
        start: output.start,
        next: output.next,
        truncated: output.truncated,
        finished: output.finished,
    }))
}

pub(super) async fn input(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session)): Path<(String, String)>,
    authority: Authority,
    Json(input): Json<CliWorkbenchTerminalInput>,
) -> Result<Json<()>, WorkbenchError> {
    authority.require()?;
    if input.bytes.len() > CLI_WORKBENCH_MAX_TERMINAL_INPUT
        || input.size.is_some_and(|size| size.rows == 0 || size.cols < 2)
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid terminal input",
        ));
    }
    let pty = terminal(tenant, project, &session)?;
    // The client sends one request at a time. Serialize writes from independent attachments too.
    let _guard = pty.input.lock().await;
    let attachment = pty.process.attach(0).map_err(io_error)?;
    if let Some(size) = input.size {
        pty.process
            .resize(&attachment, PtySize { rows: size.rows, cols: size.cols })
            .map_err(io_error)?;
    }
    let bytes = async {
        let mut remaining = input.bytes.as_slice();
        while !remaining.is_empty() {
            match pty.process.write(&attachment, remaining) {
                Ok(0) => return Err(io_error(io::ErrorKind::WriteZero.into())),
                Ok(written) => remaining = &remaining[written..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(error) => return Err(io_error(error)),
            }
        }
        Ok(Json(()))
    };
    tokio::time::timeout(Duration::from_secs(1), bytes).await.map_err(|_| {
        error(StatusCode::REQUEST_TIMEOUT, ErrorCode::TimedOut, "terminal write timed out")
    })?
}

fn io_error(cause: io::Error) -> WorkbenchError {
    match cause.kind() {
        io::ErrorKind::InvalidInput => error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid terminal cursor or size",
        ),
        io::ErrorKind::BrokenPipe => {
            error(StatusCode::GONE, ErrorCode::Closed, "terminal has closed")
        }
        _ => error(StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Disconnected, "terminal I/O failed"),
    }
}
