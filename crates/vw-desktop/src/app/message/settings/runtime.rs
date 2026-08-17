//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::update_runtime_config_async;
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::runtime::RuntimeConfig;

use super::messages::{RuntimeMessage, SettingsMessage};

fn normalize_runtime_kind(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "native" | "docker" => raw.trim().to_ascii_lowercase(),
        _ => "native".to_string(),
    }
}

fn normalize_reasoning_enabled(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        _ => "auto".to_string(),
    }
}

fn parse_reasoning_level(input: &str) -> Result<Option<String>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized = trimmed.to_ascii_lowercase().replace(['-', '_'], "");
    match normalized.as_str() {
        "minimal" | "low" | "medium" | "high" | "xhigh" => Ok(Some(normalized)),
        _ => Err("reasoning_level 仅支持 minimal / low / medium / high / xhigh".to_string()),
    }
}

fn parse_optional_u64(input: &str, field: &str) -> Result<Option<u64>, String> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value.parse::<u64>().map(Some).map_err(|_| format!("{field} 必须是非负整数"))
}

fn parse_optional_f64(input: &str, field: &str) -> Result<Option<f64>, String> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value.parse::<f64>().map_err(|_| format!("{field} 必须是数字"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{field} 必须大于 0"));
    }
    Ok(Some(parsed))
}

fn parse_csv_lines(input: &str) -> Vec<String> {
    input
        .split([',', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn persist_runtime_settings(app: &mut App) -> Result<Task<Message>, String> {
    let s = &app.runtime_settings;
    let kind = normalize_runtime_kind(&s.kind);
    let docker_image = s.docker_image.trim().to_string();
    let docker_network = s.docker_network.trim().to_string();
    let docker_read_only_rootfs = s.docker_read_only_rootfs;
    let docker_mount_workspace = s.docker_mount_workspace;
    let docker_allowed_workspace_roots = parse_csv_lines(&s.docker_allowed_workspace_roots_input);
    let docker_memory_limit_mb =
        parse_optional_u64(&s.docker_memory_limit_mb_input, "Docker 内存限制")?;
    let docker_cpu_limit = parse_optional_f64(&s.docker_cpu_limit_input, "Docker CPU 限制")?;
    let reasoning_enabled = match normalize_reasoning_enabled(&s.reasoning_enabled_input).as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    let reasoning_level = parse_reasoning_level(&s.reasoning_level_input)?;

    Ok(update_runtime_config_async(move |runtime| {
        *runtime = RuntimeConfig::default();
        runtime.kind = kind.clone();
        runtime.docker.image = if docker_image.is_empty() {
            RuntimeConfig::default().docker.image
        } else {
            docker_image
        };
        runtime.docker.network = if docker_network.is_empty() {
            RuntimeConfig::default().docker.network
        } else {
            docker_network
        };
        runtime.docker.memory_limit_mb = docker_memory_limit_mb;
        runtime.docker.cpu_limit = docker_cpu_limit;
        runtime.docker.read_only_rootfs = docker_read_only_rootfs;
        runtime.docker.mount_workspace = docker_mount_workspace;
        runtime.docker.allowed_workspace_roots = docker_allowed_workspace_roots;

        runtime.reasoning_enabled = reasoning_enabled;
        runtime.reasoning_level = reasoning_level.clone();
    }))
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Runtime(message) = message else {
        return Task::none();
    };

    if matches!(message, RuntimeMessage::Refresh) {
        app.runtime_settings.save_error = None;
        return Task::none();
    }

    match message {
        RuntimeMessage::KindChanged(value) => {
            app.runtime_settings.kind = normalize_runtime_kind(&value)
        }
        RuntimeMessage::DockerImageChanged(value) => app.runtime_settings.docker_image = value,
        RuntimeMessage::DockerNetworkChanged(value) => app.runtime_settings.docker_network = value,
        RuntimeMessage::DockerMemoryLimitMbChanged(value) => {
            app.runtime_settings.docker_memory_limit_mb_input = value
        }
        RuntimeMessage::DockerCpuLimitChanged(value) => {
            app.runtime_settings.docker_cpu_limit_input = value
        }
        RuntimeMessage::DockerReadOnlyRootfsToggled(value) => {
            app.runtime_settings.docker_read_only_rootfs = value
        }
        RuntimeMessage::DockerMountWorkspaceToggled(value) => {
            app.runtime_settings.docker_mount_workspace = value
        }
        RuntimeMessage::DockerAllowedWorkspaceRootsChanged(value) => {
            app.runtime_settings.docker_allowed_workspace_roots_input = value
        }
        RuntimeMessage::ReasoningEnabledChanged(value) => {
            app.runtime_settings.reasoning_enabled_input = normalize_reasoning_enabled(&value)
        }
        RuntimeMessage::ReasoningLevelChanged(value) => {
            app.runtime_settings.reasoning_level_input = value
        }
        RuntimeMessage::Refresh => unreachable!(),
    }

    match persist_runtime_settings(app) {
        Ok(task) => {
            app.runtime_settings.save_error = None;
            task
        }
        Err(err) => {
            app.runtime_settings.save_error = Some(err);
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
