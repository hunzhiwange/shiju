//! User and assistant message cells from the copied Codex history surface.

use super::{HistoryCell, prefixed_lines};
use crate::style::{proposed_plan_style, user_message_style};
use crate::vibe::event::V3Attachment;
use crate::wrapping::{RtOptions, adaptive_wrap_line};
use ratatui::prelude::*;

#[derive(Debug)]
pub struct UserHistoryCell {
    text: String,
    attachments: Vec<V3Attachment>,
}

impl UserHistoryCell {
    pub fn new(text: String, attachments: Vec<V3Attachment>) -> Self {
        Self { text, attachments }
    }
}

impl HistoryCell for UserHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let content_width = usize::from(width).saturating_sub(2).max(1);
        let mut lines = Vec::new();
        for source in self.text.lines().chain(self.text.is_empty().then_some("")) {
            let line = Line::from(source.to_string());
            lines.extend(
                adaptive_wrap_line(&line, RtOptions::new(content_width))
                    .into_iter()
                    .map(|line| crate::render::line_utils::line_to_static(&line)),
            );
        }
        for attachment in &self.attachments {
            let path =
                attachment.path.as_deref().map(|path| format!(" ({path})")).unwrap_or_default();
            lines.push(Line::from(format!("{}{}", attachment.label, path)));
        }
        prefixed_lines(lines, "▌ ", "  ")
            .into_iter()
            .map(|line| line.style(user_message_style()))
            .collect()
    }
}

#[derive(Debug)]
pub struct AgentMessageCell {
    markdown: String,
}

impl AgentMessageCell {
    pub fn new(markdown: String) -> Self {
        Self { markdown }
    }
}

impl HistoryCell for AgentMessageCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        crate::markdown::append_markdown(
            &self.markdown,
            Some(usize::from(width).saturating_sub(2).max(1)),
            None,
            &mut lines,
        );
        prefixed_lines(lines, "• ", "  ")
    }
}

#[derive(Debug)]
pub struct PlainHistoryCell {
    label: String,
    text: String,
    style: Style,
}

impl PlainHistoryCell {
    pub fn label(label: &str, text: &str) -> Self {
        Self { label: label.to_string(), text: text.to_string(), style: Style::default().dim() }
    }

    pub fn bullet(label: &str, text: &str) -> Self {
        Self { label: label.to_string(), text: text.to_string(), style: proposed_plan_style() }
    }

    pub fn attachment(label: &str, path: Option<&str>) -> Self {
        Self::label(
            "attachment",
            &format!("{label}{}", path.map(|p| format!(" ({p})")).unwrap_or_default()),
        )
    }

    pub fn progress(label: &str, detail: Option<&str>) -> Self {
        Self::label(label, detail.unwrap_or("working"))
    }
}

impl HistoryCell for PlainHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let prefix = format!("{}: ", self.label);
        let content_width = usize::from(width).saturating_sub(prefix.chars().count()).max(1);
        let source = Line::from(self.text.clone());
        let lines = adaptive_wrap_line(&source, RtOptions::new(content_width))
            .into_iter()
            .map(|line| crate::render::line_utils::line_to_static(&line));
        lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let prefix = if index == 0 { prefix.clone() } else { "  ".to_string() };
                Line::from(vec![
                    Span::styled(prefix, self.style),
                    Span::styled(line.to_string(), self.style),
                ])
            })
            .collect()
    }
}
