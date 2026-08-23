//! Diff presentation helpers retained from the copied Codex diff surface.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::diff_model::FileChange;
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_line;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffSummary {
    pub path: PathBuf,
    pub added: usize,
    pub removed: usize,
}

pub fn display_path_for(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}

pub fn create_diff_summary(
    changes: &BTreeMap<PathBuf, FileChange>,
    cwd: &Path,
    wrap_cols: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, (path, change)) in changes.iter().enumerate() {
        if index > 0 {
            lines.push(Line::default());
        }
        let (added, removed) = change.line_counts();
        lines.push(Line::from(vec![
            Span::styled(display_path_for(path, cwd), Style::default().bold()),
            Span::raw(format!("  +{added} -{removed}")),
        ]));
        lines.extend(render_change(change, wrap_cols));
    }
    lines
}

pub fn render_change(change: &FileChange, width: usize) -> Vec<Line<'static>> {
    let (text, style) = match change {
        FileChange::Add { content } => (content.as_str(), Color::Green),
        FileChange::Delete { content } => (content.as_str(), Color::Red),
        FileChange::Update { unified_diff, .. } => (unified_diff.as_str(), Color::Yellow),
    };
    let options = RtOptions::new(width.max(1));
    text.lines()
        .flat_map(|line| {
            let source = Line::from(line.to_string());
            adaptive_wrap_line(&source, options.clone())
                .into_iter()
                .map(|line| crate::render::line_utils::line_to_static(&line))
                .collect::<Vec<_>>()
        })
        .map(|line| line.style(Style::default().fg(style)))
        .collect()
}
