//! 文件类工具差异渲染。

use ratatui::text::{Line, Span};

use super::TuiTheme;

const MAX_DIFF_BYTES: usize = 64 * 1024;
const MAX_DIFF_LINES: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiffCardInput {
    pub(super) path: String,
    pub(super) workspace_root: Option<String>,
    pub(super) old_text: Option<String>,
    pub(super) new_text: Option<String>,
    pub(super) binary: bool,
}

pub(super) fn render_diff_card(
    input: &DiffCardInput,
    theme: TuiTheme,
    width: u16,
) -> Vec<Line<'static>> {
    let path = compact_workspace_path(input.path.as_str(), input.workspace_root.as_deref());
    let mut lines = vec![Line::styled(
        truncate_display(format!("Update {path}").as_str(), usize::from(width)),
        theme.body_style(),
    )];
    if input.binary {
        lines.push(Line::styled("  Binary file changed", theme.muted_style()));
        return lines;
    }

    let old = bounded_lines(input.old_text.as_deref().unwrap_or_default());
    let new = bounded_lines(input.new_text.as_deref().unwrap_or_default());
    for change in diff_lines(&old, &new) {
        match change {
            DiffLine::Removed { line, text } => {
                lines.push(diff_line(line, '-', text.as_str(), theme.danger, theme, width));
            }
            DiffLine::Added { line, text } => {
                lines.push(diff_line(line, '+', text.as_str(), theme.success, theme, width));
            }
            DiffLine::ContextFold(count) => {
                lines.push(Line::styled(format!("    … {count} unchanged lines"), theme.muted_style()));
            }
        }
    }
    lines
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffLine {
    Removed { line: usize, text: String },
    Added { line: usize, text: String },
    ContextFold(usize),
}

fn diff_lines(old: &[String], new: &[String]) -> Vec<DiffLine> {
    let mut lcs = vec![vec![0usize; new.len().saturating_add(1)]; old.len().saturating_add(1)];
    for old_index in (0..old.len()).rev() {
        for new_index in (0..new.len()).rev() {
            lcs[old_index][new_index] = if old[old_index] == new[new_index] {
                lcs[old_index + 1][new_index + 1].saturating_add(1)
            } else {
                lcs[old_index + 1][new_index].max(lcs[old_index][new_index + 1])
            };
        }
    }

    let mut changes = Vec::new();
    let (mut old_index, mut new_index, mut unchanged) = (0usize, 0usize, 0usize);
    while old_index < old.len() || new_index < new.len() {
        if old_index < old.len()
            && new_index < new.len()
            && old[old_index] == new[new_index]
        {
            unchanged = unchanged.saturating_add(1);
            old_index = old_index.saturating_add(1);
            new_index = new_index.saturating_add(1);
            continue;
        }
        if unchanged > 3 {
            changes.push(DiffLine::ContextFold(unchanged));
        }
        unchanged = 0;
        if new_index >= new.len()
            || (old_index < old.len()
                && lcs[old_index + 1][new_index] >= lcs[old_index][new_index + 1])
        {
            changes.push(DiffLine::Removed {
                line: old_index.saturating_add(1),
                text: old[old_index].clone(),
            });
            old_index = old_index.saturating_add(1);
        } else {
            changes.push(DiffLine::Added {
                line: new_index.saturating_add(1),
                text: new[new_index].clone(),
            });
            new_index = new_index.saturating_add(1);
        }
    }
    if unchanged > 3 {
        changes.push(DiffLine::ContextFold(unchanged));
    }
    changes
}

fn diff_line(
    number: usize,
    marker: char,
    text: &str,
    color: ratatui::style::Color,
    theme: TuiTheme,
    width: u16,
) -> Line<'static> {
    let prefix = format!("{number:>3} {marker} ");
    let available = usize::from(width).saturating_sub(prefix.chars().count());
    Line::from(vec![
        Span::styled(prefix, theme.muted_style()),
        Span::styled(truncate_display(text, available), ratatui::style::Style::default().fg(color)),
    ])
}

fn bounded_lines(text: &str) -> Vec<String> {
    let bounded = if text.len() <= MAX_DIFF_BYTES {
        text
    } else {
        let mut end = MAX_DIFF_BYTES;
        while !text.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        &text[..end]
    };
    bounded.lines().take(MAX_DIFF_LINES).map(ToOwned::to_owned).collect()
}

fn compact_workspace_path(path: &str, workspace_root: Option<&str>) -> String {
    workspace_root
        .and_then(|root| path.strip_prefix(root))
        .map(|relative| relative.trim_start_matches(['/', '\\']).to_string())
        .filter(|relative| !relative.is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn truncate_display(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut output = value.chars().take(width.saturating_sub(1)).collect::<String>();
    output.push('…');
    output
}
