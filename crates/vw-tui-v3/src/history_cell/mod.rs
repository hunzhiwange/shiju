//! History cells copied from Codex and adapted to VibeWindow display data.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::render::line_utils::prefix_lines;
use crate::vibe::event::V3DisplayEvent;

mod exec;
mod messages;
pub(crate) mod session;

pub use exec::ExecHistoryCell;
pub use messages::{AgentMessageCell, PlainHistoryCell, UserHistoryCell};
pub use session::SessionHistoryCell;

/// A single display unit in the copied Codex transcript.
pub trait HistoryCell: std::fmt::Debug + Send + Sync + std::any::Any {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>>;

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }

    fn desired_height(&self, width: u16) -> u16 {
        Paragraph::new(Text::from(self.display_lines(width)))
            .wrap(Wrap { trim: false })
            .line_count(width)
            .try_into()
            .unwrap_or(u16::MAX)
    }
}

impl dyn HistoryCell {
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Maps every VibeWindow display event to one of the copied history/exec cells.
pub fn history_cell_for(event: &V3DisplayEvent) -> Box<dyn HistoryCell> {
    match event {
        V3DisplayEvent::User { text, attachments } => {
            Box::new(UserHistoryCell::new(text.clone(), attachments.clone()))
        }
        V3DisplayEvent::Assistant { markdown } => Box::new(AgentMessageCell::new(markdown.clone())),
        V3DisplayEvent::ToolCall { call_id, tool, command } => {
            Box::new(ExecHistoryCell::from_pending(call_id.clone(), tool.clone(), command.clone()))
        }
        V3DisplayEvent::ToolResult { call_id, output, exit_code, duration_ms } => Box::new(
            ExecHistoryCell::from_result(call_id.clone(), output.clone(), *exit_code, *duration_ms),
        ),
        V3DisplayEvent::Thinking { text } => Box::new(PlainHistoryCell::bullet("thinking", text)),
        V3DisplayEvent::Step { title, status } => Box::new(PlainHistoryCell::bullet(
            "step",
            &crate::goal_display::format_step(title, *status),
        )),
        V3DisplayEvent::System { text } => Box::new(PlainHistoryCell::label("system", text)),
        V3DisplayEvent::Error { message } => Box::new(PlainHistoryCell::label("error", message)),
        V3DisplayEvent::Attachment { attachment } => {
            Box::new(PlainHistoryCell::attachment(&attachment.label, attachment.path.as_deref()))
        }
        V3DisplayEvent::Progress { label, detail } => {
            Box::new(PlainHistoryCell::progress(label, detail.as_deref()))
        }
        V3DisplayEvent::CompactBoundary { summary } => {
            Box::new(SessionHistoryCell::compact_boundary(summary))
        }
        V3DisplayEvent::Unknown { type_name, detail } => {
            Box::new(PlainHistoryCell::label(type_name, detail))
        }
    }
}

/// Renders one event using the same copied history-cell boundary used by the transcript.
pub fn render_display_event(event: &V3DisplayEvent, width: u16) -> Vec<Line<'static>> {
    history_cell_for(event).display_lines(width)
}

pub(crate) fn prefixed_lines(
    lines: Vec<Line<'static>>,
    first: &'static str,
    rest: &'static str,
) -> Vec<Line<'static>> {
    prefix_lines(lines, first.into(), rest.into())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
