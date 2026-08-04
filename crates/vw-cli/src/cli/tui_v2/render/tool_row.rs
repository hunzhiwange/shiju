//! 通用工具调用行渲染。

use ratatui::text::{Line, Span};
use serde_json::Value;

use super::TuiTheme;
use super::diff::{DiffCardInput, render_diff_card};
use super::shell::{ShellCardInput, render_shell_card};
use crate::cli::tui_v2::model::UiToolCallState;
use crate::cli::tui_v2::state::selectors::TuiToolRowView;
use crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text;

pub(super) fn render_tool_row(
    view: &TuiToolRowView,
    theme: TuiTheme,
    expanded: bool,
) -> Vec<Line<'static>> {
    let failed = view.has_error || view.state == UiToolCallState::Failed;
    let (symbol, status, color) = if failed {
        ("✗", "failed", theme.danger)
    } else {
        match view.state {
            UiToolCallState::Queued => ("●", "queued", theme.muted),
            UiToolCallState::Running => ("◐", "running", theme.accent),
            UiToolCallState::Complete => ("✓", "completed", theme.success),
            UiToolCallState::Failed => ("✗", "failed", theme.danger),
            UiToolCallState::Cancelled => ("■", "interrupted", theme.muted),
        }
    };

    let mut title = format!("{symbol} {}", view.display_name);
    if let Some(summary) = view.summary.as_ref() {
        title.push_str(format!(" ({summary})").as_str());
    }
    title.push_str(format!(" · {status}").as_str());
    if view.result_count > 1 {
        title.push_str(format!(" · {} results", view.result_count).as_str());
    }

    let mut lines = vec![Line::styled(
        sanitize_terminal_text(title.as_str()),
        ratatui::style::Style::default().fg(color),
    )];
    if let Some(specialized) = render_specialized_tool(view, theme) {
        lines.extend(specialized);
        append_tool_activity(&mut lines, view, theme);
        return lines;
    }
    if expanded {
        if let Some(arguments) = view.safe_arguments.as_ref() {
            lines.push(Line::from(vec![
                Span::styled("  input ", theme.muted_style()),
                Span::styled(sanitize_terminal_text(arguments), theme.body_style()),
            ]));
        }
        for result in &view.safe_results {
            lines.push(Line::from(vec![
                Span::styled("  result ", theme.muted_style()),
                Span::styled(sanitize_terminal_text(result), theme.body_style()),
            ]));
        }
    }
    append_tool_activity(&mut lines, view, theme);
    lines
}

fn append_tool_activity(
    lines: &mut Vec<Line<'static>>,
    view: &TuiToolRowView,
    theme: TuiTheme,
) {
    for progress in &view.progress_labels {
        lines.push(Line::styled(
            sanitize_terminal_text(format!("  ↳ {progress}").as_str()),
            theme.muted_style(),
        ));
    }
    for notice in &view.notice_labels {
        lines.push(Line::styled(
            sanitize_terminal_text(format!("  ↳ {notice}").as_str()),
            theme.muted_style(),
        ));
    }
}

fn render_specialized_tool(
    view: &TuiToolRowView,
    theme: TuiTheme,
) -> Option<Vec<Line<'static>>> {
    match view.tool_name.trim().to_ascii_lowercase().as_str() {
        "edit" | "write" | "apply_patch" | "notebook_edit" => {
            let arguments = parse_object(view.safe_arguments.as_deref()?)?;
            let path = text_field(&arguments, &["file_path", "path"])
                .map(|value| sanitize_terminal_text(value.as_str()))
                .unwrap_or_else(|| "[invalid path]".to_string());
            let old_text = text_field(&arguments, &["old_string", "old_text"])
                .map(|value| sanitize_terminal_text(value.as_str()));
            let new_text = text_field(&arguments, &["new_string", "new_text", "content"])
                .map(|value| sanitize_terminal_text(value.as_str()));
            let binary = arguments.get("binary").and_then(Value::as_bool).unwrap_or(false);
            Some(render_diff_card(
                &DiffCardInput { path, workspace_root: None, old_text, new_text, binary },
                theme,
                80,
            ))
        }
        "bash" | "shell" | "powershell" | "pwsh" => {
            let arguments = parse_object(view.safe_arguments.as_deref()?)?;
            let result = view.safe_results.last().and_then(|result| parse_object(result));
            Some(render_shell_card(
                &ShellCardInput {
                    command: text_field(&arguments, &["command", "cmd"])
                        .map(|value| sanitize_terminal_text(value.as_str()))
                        .unwrap_or_default(),
                    cwd: text_field(&arguments, &["cwd"])
                        .map(|value| sanitize_terminal_text(value.as_str())),
                    stdout: result
                        .as_ref()
                        .and_then(|value| text_field(value, &["stdout", "output"]))
                        .or_else(|| {
                            view.safe_results
                                .last()
                                .filter(|result| parse_object(result).is_none())
                                .cloned()
                        })
                        .map(|value| sanitize_terminal_text(value.as_str()))
                        .unwrap_or_default(),
                    stderr: result
                        .as_ref()
                        .and_then(|value| text_field(value, &["stderr", "error"]))
                        .map(|value| sanitize_terminal_text(value.as_str()))
                        .unwrap_or_default(),
                    exit_code: result
                        .as_ref()
                        .and_then(|value| value.get("exit_code").or_else(|| value.get("code")))
                        .and_then(Value::as_i64)
                        .and_then(|code| i32::try_from(code).ok()),
                    duration_ms: result
                        .as_ref()
                        .and_then(|value| value.get("duration_ms"))
                        .and_then(Value::as_u64),
                    background: arguments
                        .get("background")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    running: view.state == UiToolCallState::Running,
                },
                theme,
                80,
            ))
        }
        _ => None,
    }
}

fn parse_object(value: &str) -> Option<serde_json::Map<String, Value>> {
    let value = serde_json::from_str::<Value>(value).ok()?;
    let Value::Object(entries) = value else {
        return None;
    };
    Some(entries)
}

fn text_field(
    value: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}
