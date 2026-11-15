//! 跨平台子进程启动参数的组装辅助。

use std::collections::HashMap;
use std::ffi::OsString;
use tokio::process::Command;
#[cfg(not(windows))]
use vw_shared::shell::tokio_command;

#[cfg(test)]
#[path = "spawn_command_options_tests.rs"]
mod spawn_command_options_tests;

#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
fn merged_windows_env(env_overrides: &HashMap<String, String>) -> HashMap<String, OsString> {
    let mut env = HashMap::new();
    for (key, value) in std::env::vars_os() {
        env.insert(key.to_string_lossy().to_uppercase(), value);
    }
    for (key, value) in env_overrides {
        env.insert(key.to_uppercase(), OsString::from(value));
    }
    env
}

#[cfg(windows)]
fn resolve_windows_command(command: &Path, env: &HashMap<String, OsString>) -> Option<PathBuf> {
    let extensions = env
        .get("PATHEXT")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let command_extension = command
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_ascii_lowercase()));
    let candidates = if command_extension.is_some() {
        vec![command.to_path_buf()]
    } else {
        extensions
            .iter()
            .map(|extension| {
                let mut candidate = command.as_os_str().to_os_string();
                candidate.push(extension);
                PathBuf::from(candidate)
            })
            .collect::<Vec<_>>()
    };
    let command_text = command.as_os_str().to_string_lossy();
    let has_path =
        command.is_absolute() || command_text.contains('/') || command_text.contains('\\');

    if has_path {
        return candidates.into_iter().find(|candidate| candidate.exists());
    }

    let path_value = env.get("PATH")?;
    for directory in std::env::split_paths(path_value) {
        for candidate in &candidates {
            let resolved = directory.join(candidate);
            if resolved.exists() {
                return Some(resolved);
            }
        }
    }

    None
}

#[cfg(windows)]
fn should_use_windows_batch_shell(command: &Path, env: &HashMap<String, OsString>) -> bool {
    let resolved_command =
        resolve_windows_command(command, env).unwrap_or_else(|| command.to_path_buf());
    matches!(
        resolved_command
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .as_deref(),
        Some("cmd" | "bat")
    )
}

pub fn build_spawn_command(
    command_name: impl Into<OsString>,
    env_overrides: &HashMap<String, String>,
) -> Command {
    let command_name = command_name.into();

    #[cfg(windows)]
    let mut command = {
        let merged_env = merged_windows_env(env_overrides);
        let command_path = PathBuf::from(&command_name);
        if should_use_windows_batch_shell(&command_path, &merged_env) {
            let resolved =
                resolve_windows_command(&command_path, &merged_env).unwrap_or(command_path);
            let mut shell_command = Command::new("cmd");
            shell_command.arg("/C").arg(resolved);
            shell_command
        } else {
            Command::new(&command_name)
        }
    };

    #[cfg(not(windows))]
    let mut command = tokio_command(command_name.to_string_lossy().as_ref());

    for (key, value) in env_overrides {
        command.env(key, value);
    }

    command
}
