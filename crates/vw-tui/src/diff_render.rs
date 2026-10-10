//! 从复制的 Codex 差异界面保留下来的差异展示辅助函数。
//! Diff presentation helpers retained from the copied Codex diff surface.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::vibe::event::{V3FileChange, V3FileDiffEntry};
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_line;

pub fn create_diff_summary(files: &[V3FileDiffEntry], wrap_cols: usize) -> Vec<Line<'static>> {
    // 文件之间插入空行，并将路径摘要放在对应差异正文之前。
    let mut lines = Vec::new();
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            lines.push(Line::default());
        }
        let (added, removed) = line_counts(&file.change);
        lines.push(Line::from(vec![
            Span::styled(file.path.clone(), Style::default().bold()),
            Span::raw("  "),
            Span::styled(format!("+{added}"), Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled(format!("-{removed}"), Style::default().fg(Color::Red)),
        ]));
        lines.extend(render_change(&file.change, wrap_cols));
    }
    lines
}

fn line_counts(change: &V3FileChange) -> (usize, usize) {
    match change {
        V3FileChange::Add { content } => (content.lines().count(), 0),
        V3FileChange::Delete { content } => (0, content.lines().count()),
        V3FileChange::Update { unified_diff } => unified_diff
            .lines()
            .filter(|line| !line.starts_with("+++") && !line.starts_with("---"))
            .fold((0, 0), |(added, removed), line| {
                if line.starts_with('+') {
                    (added + 1, removed)
                } else if line.starts_with('-') {
                    (added, removed + 1)
                } else {
                    (added, removed)
                }
            }),
    }
}

pub fn render_change(change: &V3FileChange, width: usize) -> Vec<Line<'static>> {
    let text = match change {
        V3FileChange::Add { content } | V3FileChange::Delete { content } => content,
        V3FileChange::Update { unified_diff } => unified_diff,
    };
    let options = RtOptions::new(width.max(1));
    text.lines()
        .flat_map(|line| {
            let color = match change {
                V3FileChange::Add { .. } => Some(Color::Green),
                V3FileChange::Delete { .. } => Some(Color::Red),
                V3FileChange::Update { .. } if line.starts_with('+') => Some(Color::Green),
                V3FileChange::Update { .. } if line.starts_with('-') => Some(Color::Red),
                V3FileChange::Update { .. } if line.starts_with("@@") => Some(Color::Cyan),
                V3FileChange::Update { .. } => None,
            };
            let style = color.map_or_else(Style::default, |color| Style::default().fg(color));
            let source = Line::styled(line.to_string(), style);
            adaptive_wrap_line(&source, options.clone())
                .into_iter()
                .map(|line| crate::render::line_utils::line_to_static(&line))
                .collect::<Vec<_>>()
        })
        .collect()
}
