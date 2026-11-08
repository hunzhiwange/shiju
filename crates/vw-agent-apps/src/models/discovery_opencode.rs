//! Read the authorized execution host's OpenCode runtime catalog.

use std::{collections::BTreeSet, future::Future, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

use super::discovery_process;
use crate::{
    apps::{
        discovery::read::{LocalReadEnvironment, locate_cli},
        launch::{host, request::LaunchMaterial},
    },
    providers::paths,
};

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
    let output = discovery_process::output(cancelled, || command(host, directory_override)).await?;
    Ok(parse(&output))
}

fn command(
    host: &LocalReadEnvironment,
    directory_override: Option<&str>,
) -> Result<std::process::Command, &'static str> {
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
    let mut command =
        host::command(material, &home, &host.search_paths).map_err(|_| "unavailable")?;
    command.env("OPENCODE_CONFIG_DIR", directory).env("OPENCODE_DISABLE_PROJECT_CONFIG", "true");
    Ok(command)
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
