use ratatui::text::Line;

use super::TuiTheme;
use super::diff::{DiffCardInput, render_diff_card};
use super::shell::{ShellCardInput, render_shell_card};
use super::tool_row::render_tool_row;
use crate::cli::tui_v2::model::UiToolCallState;
use crate::cli::tui_v2::state::selectors::TuiToolRowView;

#[test]
fn tui_diff_and_shell_cards_match_reference_frames() {
    let theme = TuiTheme::default();
    let diff = render_diff_card(
        &DiffCardInput {
            path: "/workspace/project/src/very_long_component_name.rs".to_string(),
            workspace_root: Some("/workspace/project".to_string()),
            old_text: Some("line one\nsame\nold value\nlast".to_string()),
            new_text: Some(
                "line one\nsame\nnew value that is deliberately too long for the viewport\nlast\nadded"
                    .to_string(),
            ),
            binary: false,
        },
        theme,
        38,
    );
    assert_eq!(
        line_texts(&diff),
        vec![
            "Update src/very_long_component_name.rs",
            "  3 - old value",
            "  3 + new value that is deliberately …",
            "  5 + added",
        ]
    );
    assert_eq!(diff[1].spans[1].style.fg, Some(theme.danger));
    assert_eq!(diff[2].spans[1].style.fg, Some(theme.success));

    let binary = render_diff_card(
        &DiffCardInput {
            path: "assets/logo.bin".to_string(),
            workspace_root: None,
            old_text: None,
            new_text: None,
            binary: true,
        },
        theme,
        30,
    );
    assert_eq!(line_texts(&binary), vec!["Update assets/logo.bin", "  Binary file changed"]);

    let long_stdout = (0..14)
        .map(|index| format!("stdout-{index:02} {}", "x".repeat(20)))
        .collect::<Vec<_>>()
        .join("\n");
    let shell = render_shell_card(
        &ShellCardInput {
            command: "cargo test -p vw-cli".to_string(),
            cwd: Some("/workspace/project".to_string()),
            stdout: long_stdout,
            stderr: "warning: simulated stderr".to_string(),
            exit_code: Some(1),
            duration_ms: Some(1_234),
            background: true,
            running: false,
        },
        theme,
        34,
    );
    let shell_text = line_texts(&shell);
    assert_eq!(shell_text[0], "$ cargo test -p vw-cli");
    assert_eq!(shell_text[1], "  cwd /workspace/project");
    assert!(shell_text.iter().any(|line| line.starts_with("! warning: simulated stderr")));
    assert!(shell_text.iter().any(|line| line == "… 4 lines truncated (Ctrl+O to expand)"));
    assert_eq!(shell_text.last().map(String::as_str), Some("exit 1 · 1.2s · background"));
    assert!(shell.len() <= 15, "line cap includes chrome: {}", shell.len());
    assert_eq!(
        shell
            .iter()
            .find(|line| line_text(line).starts_with("! "))
            .and_then(|line| line.spans.first())
            .and_then(|span| span.style.fg),
        Some(theme.danger)
    );

    let byte_limited = render_shell_card(
        &ShellCardInput {
            command: "emit".to_string(),
            cwd: None,
            stdout: "界".repeat(2_000),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: None,
            background: false,
            running: false,
        },
        theme,
        40,
    );
    assert!(line_texts(&byte_limited).iter().any(|line| line.contains("bytes truncated")));

    let running = render_shell_card(
        &ShellCardInput {
            command: "watch".to_string(),
            cwd: None,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: Some(900),
            background: true,
            running: true,
        },
        theme,
        20,
    );
    assert_eq!(line_texts(&running), vec!["$ watch", "running · 0.9s · background"]);

    let specialized = render_tool_row(
        &TuiToolRowView {
            tool_name: "edit".to_string(),
            display_name: "Update".to_string(),
            summary: None,
            state: UiToolCallState::Complete,
            result_count: 1,
            has_error: false,
            safe_arguments: Some(
                r#"{"file_path":"src/lib.rs","old_string":"old","new_string":"new"}"#
                    .to_string(),
            ),
            safe_results: vec!["ok".to_string()],
            progress_labels: Vec::new(),
            notice_labels: Vec::new(),
        },
        theme,
        false,
    );
    assert!(line_texts(&specialized).iter().any(|line| line == "Update src/lib.rs"));
}

fn line_texts(lines: &[Line<'static>]) -> Vec<String> {
    lines.iter().map(line_text).collect()
}

fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().fold(String::new(), |mut out, span| {
        out.push_str(span.content.as_ref());
        out
    })
}
