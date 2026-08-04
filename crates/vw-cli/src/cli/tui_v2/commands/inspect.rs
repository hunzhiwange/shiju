use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use super::super::model::{UiOverlay, UiReportOverlay};
use super::super::state::{TuiState, select_status_summary};
use super::super::transcript_controller::transcript_plain_text_lines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectCommand {
    Status,
    Context,
    Usage,
    Cost,
    Diff,
    Files,
    Stats,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectReport {
    pub(crate) title: String,
    pub(crate) lines: Vec<String>,
    pub(crate) capability_notice: Option<String>,
}

pub(crate) fn inspect_session(state: &TuiState, command: InspectCommand) -> InspectReport {
    let status = select_status_summary(state);
    match command {
        InspectCommand::Status => InspectReport {
            title: "会话状态".to_string(),
            lines: vec![
                format!("session: {}", state.session.session_id.as_deref().unwrap_or("-")),
                format!("provider: {}", state.status.provider_name.as_deref().unwrap_or("-")),
                format!("model: {}", state.status.model_name.as_deref().unwrap_or("-")),
            ],
            capability_notice: None,
        },
        InspectCommand::Context => InspectReport {
            title: "上下文".to_string(),
            lines: vec![
                format!(
                    "workspace: {}",
                    state
                        .project
                        .workspace_root
                        .as_ref()
                        .map_or_else(|| "-".to_string(), |path| path.display().to_string())
                ),
                format!("scope: {}", state.session.scope.as_deref().unwrap_or("-")),
                format!("messages: {}", state.messages.len()),
            ],
            capability_notice: None,
        },
        InspectCommand::Cost => InspectReport {
            title: "用量".to_string(),
            lines: vec![
                format!("input tokens: {}", status.token_usage.input_tokens),
                format!("output tokens: {}", status.token_usage.output_tokens),
                format!("reasoning tokens: {}", status.token_usage.reasoning_tokens),
            ],
            capability_notice: Some(
                "当前 provider 未返回统一价格表；不伪造 Claude 账号计费或美元成本".to_string(),
            ),
        },
        InspectCommand::Usage => InspectReport {
            title: "用量与统计".to_string(),
            lines: vec![
                format!("input tokens: {}", status.token_usage.input_tokens),
                format!("output tokens: {}", status.token_usage.output_tokens),
                format!("reasoning tokens: {}", status.token_usage.reasoning_tokens),
                format!("messages: {}", status.message_count),
                format!("steps: {}", status.step_count),
            ],
            capability_notice: Some(
                "仅显示网关返回的真实 token 与步骤；不提供 Claude 账号额度或推测金额".to_string(),
            ),
        },
        InspectCommand::Diff => InspectReport {
            title: "变更".to_string(),
            lines: state.project.git_status.modified_files().to_vec(),
            capability_notice: Some(
                "当前只读数据面提供修改文件清单，不提供可核验的完整 git diff".to_string(),
            ),
        },
        InspectCommand::Files => InspectReport {
            title: "修改文件".to_string(),
            lines: state.project.git_status.modified_files().to_vec(),
            capability_notice: None,
        },
        InspectCommand::Stats => InspectReport {
            title: "会话统计".to_string(),
            lines: vec![
                format!("messages: {}", status.message_count),
                format!("steps: {}", status.step_count),
                format!("pending questions: {}", status.pending_questions),
                format!("todos: {}", status.todo_count),
            ],
            capability_notice: None,
        },
        InspectCommand::Export => InspectReport {
            title: "Transcript".to_string(),
            lines: transcript_plain_text_lines(&state.messages),
            capability_notice: None,
        },
    }
}

pub(crate) fn execute_inspect_command(
    state: &mut TuiState,
    command: InspectCommand,
    argument: Option<&str>,
) -> Result<(), String> {
    if command == InspectCommand::Export {
        let raw_path = argument
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| "用法: /export <path>".to_string())?;
        let requested = std::path::PathBuf::from(raw_path);
        let path = if requested.is_absolute() {
            requested
        } else {
            state
                .project
                .workspace_root
                .as_ref()
                .map_or(requested.clone(), |root| root.join(requested))
        };
        export_transcript(state, path.as_path()).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("目标文件 already exists，拒绝覆盖: {}", path.display())
            } else {
                format!("无法导出到 {}: {error}", path.display())
            }
        })?;
        state.overlays.push(UiOverlay::Report(UiReportOverlay {
            title: "导出完成".to_string(),
            lines: vec![path.display().to_string()],
            capability_notice: Some("文件以不覆盖模式创建；Unix 权限为 0600".to_string()),
            scroll_offset: 0,
        }));
        return Ok(());
    }

    if argument.is_some() {
        return Err("该检查命令不接受参数".to_string());
    }
    let report = inspect_session(state, command);
    state.overlays.push(UiOverlay::Report(UiReportOverlay {
        title: report.title,
        lines: report.lines,
        capability_notice: report.capability_notice,
        scroll_offset: 0,
    }));
    Ok(())
}

pub(crate) fn export_transcript(state: &TuiState, path: &Path) -> std::io::Result<()> {
    let report = inspect_session(state, InspectCommand::Export);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(report.lines.join("\n").as_bytes())
}
