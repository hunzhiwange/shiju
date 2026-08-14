//! 配置查看与初始化命令的处理逻辑。

use std::io::{self, Write};

use serde::Serialize;

use crate::cli::flags::GlobalFlags;
use crate::cli::json_output::emit_json_result;
use crate::{
    ConfigDisplay, ConfigError, InitGlobalConfigFileResult, ResolvedAcpxConfig,
    init_global_config_file, to_config_display,
};

#[derive(Debug, thiserror::Error)]
pub enum ConfigCommandError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Config(#[from] ConfigError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigPaths {
    global: String,
    project: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LoadedConfigSources {
    global: bool,
    project: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigShowPayload {
    #[serde(flatten)]
    display: ConfigDisplay,
    paths: ConfigPaths,
    loaded: LoadedConfigSources,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigInitPayload {
    path: String,
    created: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCommand {
    Show,
    Init,
}

fn config_show_payload(config: &ResolvedAcpxConfig) -> ConfigShowPayload {
    ConfigShowPayload {
        display: to_config_display(config),
        paths: ConfigPaths {
            global: config.global_path.clone(),
            project: config.project_path.clone(),
        },
        loaded: LoadedConfigSources {
            global: config.has_global_config,
            project: config.has_project_config,
        },
    }
}

fn config_init_payload(result: InitGlobalConfigFileResult) -> ConfigInitPayload {
    ConfigInitPayload { path: result.path, created: result.created }
}

pub fn write_config_show<W: Write>(
    stdout: &mut W,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), ConfigCommandError> {
    let payload = config_show_payload(config);

    if emit_json_result(stdout, global_flags.format, &payload)? {
        return Ok(());
    }

    serde_json::to_writer_pretty(&mut *stdout, &payload).map_err(io::Error::other)?;
    stdout.write_all(b"\n")?;
    Ok(())
}

pub async fn write_config_init<W: Write>(
    stdout: &mut W,
    global_flags: &GlobalFlags,
) -> Result<(), ConfigCommandError> {
    let result = config_init_payload(init_global_config_file().await?);
    write_config_init_result(stdout, global_flags, &result)
}

fn write_config_init_result<W: Write>(
    stdout: &mut W,
    global_flags: &GlobalFlags,
    result: &ConfigInitPayload,
) -> Result<(), ConfigCommandError> {
    if emit_json_result(stdout, global_flags.format, &result)? {
        return Ok(());
    }

    if global_flags.format == crate::OutputFormat::Quiet {
        writeln!(stdout, "{}", result.path)?;
        return Ok(());
    }

    if result.created {
        writeln!(stdout, "Created {}", result.path)?;
        return Ok(());
    }

    writeln!(stdout, "Config already exists: {}", result.path)?;
    Ok(())
}

pub fn handle_config_show(
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), ConfigCommandError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_config_show(&mut stdout, global_flags, config)
}

pub async fn handle_config_init(global_flags: &GlobalFlags) -> Result<(), ConfigCommandError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_config_init(&mut stdout, global_flags).await
}

pub async fn handle_config_command(
    command: ConfigCommand,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<(), ConfigCommandError> {
    match command {
        ConfigCommand::Show => handle_config_show(global_flags, config),
        ConfigCommand::Init => handle_config_init(global_flags).await,
    }
}

#[cfg(test)]
#[path = "config_command_tests.rs"]
mod config_command_tests;
