//! Blocking process execution inside the host lifecycle write lock.
use super::{coordinator::ToolLifecycleCoordinator, native_primary::InstallEnvironment, update};
use crate::apps::versions::node::NodeRequirement;
use std::io::Read;
use std::process::{Command, Stdio};
use vw_api_types::agent_apps::{AppId, AppInstallationReport};

#[derive(Debug)]
pub struct ProcessResult {
    pub exit_code: Option<i32>,
    /// Installer output is untrusted and may contain credentials. Only bounded,
    /// content-free diagnostics leave the executor.
    pub output: String,
    pub installations: AppInstallationReport,
}

/// Host callbacks perform fresh discovery and Node checks, never client input.
/// All callbacks and the child wait execute under the same host write lock.
pub async fn install<G, P>(
    app: AppId,
    environment: InstallEnvironment,
    gate: G,
    probe: P,
) -> Result<ProcessResult, String>
where
    G: FnOnce() -> Result<NodeRequirement, String> + Send + 'static,
    P: FnOnce() -> Result<AppInstallationReport, String> + Send + 'static,
{
    ToolLifecycleCoordinator
        .run(vec![app], move || {
            match environment {
                InstallEnvironment::Windows if !cfg!(windows) => {
                    return Err("Windows executor required".into());
                }
                InstallEnvironment::Posix if cfg!(windows) => {
                    return Err("POSIX executor required".into());
                }
                InstallEnvironment::Wsl if !cfg!(target_os = "linux") => {
                    return Err("executor inside authorized WSL distro required".into());
                }
                _ => {}
            }
            let recipe = install_recipe(app, environment)?;
            check_node(gate()?)?;
            let command = command(recipe.program, &recipe.args, &[], None)?;
            finish(command, app, probe)
        })
        .await
}

/// Rebuild task 19's plan from fresh host observations before any child starts.
pub async fn upgrade<R, G, P>(
    app: AppId,
    preview: update::UpdatePlan,
    confirmed: bool,
    refresh: R,
    gate: G,
    probe: P,
) -> Result<ProcessResult, String>
where
    R: FnOnce() -> Result<(AppInstallationReport, Vec<String>), String> + Send + 'static,
    G: FnOnce() -> Result<NodeRequirement, String> + Send + 'static,
    P: FnOnce() -> Result<AppInstallationReport, String> + Send + 'static,
{
    ToolLifecycleCoordinator
        .run(vec![app], move || {
            if preview.app_id() != app {
                return Err("plan application mismatch".into());
            }
            let (report, companions) = refresh()?;
            preview.revalidate(&report, &companions, confirmed).map_err(str::to_owned)?;
            check_node(gate()?)?;
            let c = &preview.command;
            finish(command(&c.program, &c.args, &c.env, Some(&c.path_prefix))?, app, || {
                let report = probe()?;
                if !preview.verify_result(&report) {
                    return Err("selected installation postcondition failed".into());
                }
                Ok(report)
            })
        })
        .await
}

fn check_node(requirement: NodeRequirement) -> Result<(), String> {
    if matches!(requirement, NodeRequirement::Gap { .. }) {
        return Err("Node upgrade required".into());
    }
    Ok(())
}

pub(crate) fn command(
    program: &str,
    args: &[String],
    env: &[(String, String)],
    prefix: Option<&str>,
) -> Result<Command, String> {
    let mut command = Command::new(if program == "bash" { "/bin/bash" } else { program });
    if program == "bash" || program == "/bin/bash" {
        command.args(["-o", "pipefail"]);
    }
    command.args(args).envs(env.iter().cloned());
    if let Some(prefix) = prefix {
        let mut paths = vec![std::path::PathBuf::from(prefix)];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
        command.env("PATH", std::env::join_paths(paths).map_err(|_| "invalid host PATH")?);
    }
    Ok(command)
}

/// Conservative batch literals: reject expansion and shell metacharacters.
/// Rules are platform independent so Windows fixtures run on macOS too.
#[cfg(any(windows, test))]
pub(super) fn windows_batch(program: &str, args: &[String]) -> Result<String, String> {
    let quote = |s: &str| {
        if s.contains(['%', '!', '^', '&', '|', '<', '>', '\"', '\r', '\n', '\0']) {
            Err("unsafe Windows batch argument".to_owned())
        } else {
            Ok(format!("\"{s}\""))
        }
    };
    let mut line = format!("@echo off\r\nchcp 65001 >nul\r\ncall {}", quote(program)?);
    for arg in args {
        line.push(' ');
        line.push_str(&quote(arg)?);
    }
    line.push_str("\r\nexit /b %errorlevel%\r\n");
    Ok(line)
}

fn drain(mut reader: impl Read) -> std::io::Result<usize> {
    let mut buffer = [0; 4096];
    let mut retained = 0;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            return Ok(retained);
        }
        retained = (retained + n).min(8192);
    }
}

fn finish(
    command: Command,
    app: AppId,
    probe: impl FnOnce() -> Result<AppInstallationReport, String>,
) -> Result<ProcessResult, String> {
    execute(command, app, probe)
}

pub(super) fn execute(
    command: Command,
    app: AppId,
    probe: impl FnOnce() -> Result<AppInstallationReport, String>,
) -> Result<ProcessResult, String> {
    let (exit_code, output) = run_command(command)?;
    let installations = probe()?;
    if !installations.installations.iter().any(|i| {
        i.runtime_status == vw_api_types::agent_apps::AppInstallationRuntimeStatus::Runnable
    }) || installations.installations.iter().any(|i| i.app_id != app)
    {
        return Err("installation postcondition failed".into());
    }
    Ok(ProcessResult { exit_code, output, installations })
}

pub(crate) fn run_command(mut command: Command) -> Result<(Option<i32>, String), String> {
    #[cfg(windows)]
    let _batch = {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        let program = command.get_program().to_str().ok_or("invalid Windows program")?;
        if program.to_ascii_lowercase().ends_with(".cmd") || program == "npm" {
            let args = command
                .get_args()
                .map(|a| a.to_str().map(str::to_owned).ok_or("invalid Windows argument"))
                .collect::<Result<Vec<_>, _>>()?;
            let script = windows_batch(program, &args)?;
            let mut file = tempfile::Builder::new()
                .suffix(".bat")
                .tempfile()
                .map_err(|_| "batch creation failed")?;
            file.write_all(script.as_bytes()).map_err(|_| "batch write failed")?;
            let env = command
                .get_envs()
                .map(|(k, v)| (k.to_owned(), v.map(|v| v.to_owned())))
                .collect::<Vec<_>>();
            let mut wrapper = Command::new("cmd.exe");
            wrapper.args(["/D", "/C"]).arg(file.path()).creation_flags(0x08000000);
            for (k, v) in env {
                if let Some(v) = v {
                    wrapper.env(k, v);
                } else {
                    wrapper.env_remove(k);
                }
            }
            command = wrapper;
            Some(file)
        } else {
            None
        }
    };
    let operation = super::operations::current();
    if operation.as_ref().is_some_and(|op| op.is_cancelled()) {
        return Err("operation cancelled".into());
    }
    #[cfg(not(unix))]
    if operation.is_some() {
        return Err("cancellable lifecycle execution requires a process-tree executor".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    super::operations::phase(super::operations::Phase::Executing);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "installer could not start")?;
    let stdout = child.stdout.take().ok_or("installer stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("installer stderr unavailable")?;
    std::thread::scope(|scope| {
        let out = scope.spawn(|| drain(stdout));
        let err = scope.spawn(|| drain(stderr));
        let mut cancelled = false;
        let mut exited = None;
        let status = loop {
            if !cancelled && operation.as_ref().is_some_and(|op| op.is_cancelled()) {
                cancelled = true;
                #[cfg(unix)]
                {
                    // The child owns this fresh group; never signal the host group.
                    let result = unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                    if result != 0
                        && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
                    {
                        // Keep holding the write lock rather than claim cancellation.
                        let _ = child.kill();
                    }
                }
            }
            if exited.is_none() {
                exited = child.try_wait().map_err(|_| "installer wait failed")?;
            }
            if out.is_finished()
                && err.is_finished()
                && let Some(status) = exited
            {
                break status;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let out = out
            .join()
            .map_err(|_| "installer output reader failed")?
            .map_err(|_| "installer output read failed")?;
        let err = err
            .join()
            .map_err(|_| "installer output reader failed")?
            .map_err(|_| "installer output read failed")?;
        if cancelled {
            return Err("operation cancelled".into());
        }
        super::operations::phase(super::operations::Phase::Postflight);
        if !status.success() {
            return Err(format!("installer failed (exit {:?}; output redacted)", status.code()));
        }
        Ok((status.code(), format!("stdout: {out} bytes; stderr: {err} bytes (redacted, capped)")))
    })
}

pub(super) fn install_recipe(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<super::native_primary::InstallCommand, String> {
    match app {
        AppId::Claude | AppId::OpenCode => super::native_primary::install_command(app, environment),
        AppId::GrokBuild | AppId::Mcode => {
            super::native_grok_mcode::install_command(app, environment)
        }
        AppId::Hermes | AppId::Agy => super::native_hermes_agy::install_command(app, environment),
        AppId::Kimi | AppId::Omp => super::native_kimi_omp::install_command(app, environment),
        _ => super::npm::install_args(app).map(|args| super::native_primary::InstallCommand {
            program: "npm",
            args: args.iter().map(|a| (*a).into()).collect(),
        }),
    }
    .map_err(str::to_owned)
}

/// Child-only environment; never mutates the gateway process environment.
pub struct WriteProcessEnvironment {
    home: std::path::PathBuf,
    prefix: std::path::PathBuf,
    tools: std::path::PathBuf,
}
impl WriteProcessEnvironment {
    pub(crate) fn new(
        home: std::path::PathBuf,
        prefix: std::path::PathBuf,
        tools: std::path::PathBuf,
    ) -> Self {
        Self { home, prefix, tools }
    }

    pub(crate) fn apply(&self, command: &mut std::process::Command) -> Result<(), String> {
        command
            .env_clear()
            .env("HOME", &self.home)
            .env(
                "PATH",
                std::env::join_paths([
                    self.tools.as_path(),
                    std::path::Path::new("/usr/bin"),
                    std::path::Path::new("/bin"),
                ])
                .map_err(|_| "invalid PATH")?,
            )
            .env("npm_config_prefix", &self.prefix)
            .env("npm_config_engine_strict", "true")
            .env("npm_config_userconfig", "/dev/null")
            .env("npm_config_globalconfig", "/dev/null")
            .current_dir(&self.prefix);
        Ok(())
    }
}
