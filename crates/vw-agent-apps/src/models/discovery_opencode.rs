//! Read the authorized execution host's OpenCode runtime catalog.

use std::{collections::BTreeSet, future::Future, path::Path, process::Stdio, time::Duration};

use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

use crate::{
    apps::{
        discovery::read::{LocalReadEnvironment, locate_cli},
        launch::{host, request::LaunchMaterial},
    },
    providers::paths,
};

const TIMEOUT: Duration = Duration::from_secs(20);
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct OpenCodeModelRef {
    pub provider_id: String,
    pub model_id: String,
}

/// The caller authorizes the host and loads its saved OpenCode directory override.
/// Cancellation uses the caller's task signal and waits for the CLI to exit.
pub async fn discover(
    host: &LocalReadEnvironment,
    directory_override: Option<&str>,
    cancelled: impl Future<Output = ()>,
) -> Result<Vec<OpenCodeModelRef>, &'static str> {
    let mut child = None;
    let result = tokio::select! {
        biased;
        () = cancelled => Err("cancelled"),
        result = tokio::time::timeout(TIMEOUT, run(host, directory_override, &mut child)) => {
            result.unwrap_or(Err("timed_out"))
        }
    };
    if result.is_err()
        && let Some(mut child) = child
    {
        child.kill_group();
        let _ = child.0.kill().await;
    }
    result
}

struct ModelProcess(Child);

impl ModelProcess {
    fn kill_group(&self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: the unreaped CLI owns the new group created by process_group(0).
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
    }
}

impl Drop for ModelProcess {
    fn drop(&mut self) {
        self.kill_group();
    }
}

async fn run(
    host: &LocalReadEnvironment,
    directory_override: Option<&str>,
    process: &mut Option<ModelProcess>,
) -> Result<Vec<OpenCodeModelRef>, &'static str> {
    let program = locate_cli(AppId::OpenCode, host).map_err(|_| "unavailable")?;
    let home = host.home.canonicalize().map_err(|_| "unavailable")?;
    let config = paths::read(&home, AppId::OpenCode, directory_override, &Default::default())
        .map_err(|_| "invalid_config")?;
    if config.files.iter().any(|file| file.state == ConfigPathState::Inaccessible) {
        return Err("invalid_config");
    }
    let directory = Path::new(&config.directory);
    let cwd = match paths::open_beneath(&home, directory, true) {
        Ok(_) => directory,
        Err(ConfigPathState::Absent) => &home,
        Err(_) => return Err("invalid_config"),
    };
    let material = LaunchMaterial {
        program,
        cwd: cwd.to_path_buf(),
        args: vec!["models".into()],
        skip_permissions: false,
    };
    let command = host::command(material, &home, &host.search_paths).map_err(|_| "unavailable")?;
    let child = Command::from(command)
        .env("OPENCODE_CONFIG_DIR", directory)
        .env("OPENCODE_DISABLE_PROJECT_CONFIG", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "unavailable")?;
    let child = &mut process.insert(ModelProcess(child)).0;
    let stdout = child.stdout.take().ok_or("unavailable")?;
    let mut bytes = Vec::new();
    stdout
        .take((OUTPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "unavailable")?;
    if bytes.len() > OUTPUT_LIMIT {
        return Err("invalid_response");
    }
    if !child.wait().await.map_err(|_| "unavailable")?.success() {
        return Err("unavailable");
    }
    let output = std::str::from_utf8(&bytes).map_err(|_| "invalid_response")?;
    Ok(parse(output))
}

fn parse(output: &str) -> Vec<OpenCodeModelRef> {
    output
        .lines()
        .filter(|line| !line.chars().any(char::is_control))
        .filter_map(|line| {
            let (provider, model) = line.trim().split_once('/')?;
            if provider.is_empty()
                || !provider.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                || model.is_empty()
                || model.chars().any(char::is_whitespace)
            {
                return None;
            }
            Some((provider, model))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(provider, model)| OpenCodeModelRef {
            provider_id: provider.into(),
            model_id: model.into(),
        })
        .collect()
}
