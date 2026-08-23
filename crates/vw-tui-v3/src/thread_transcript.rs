//! Render VibeWindow display events into transcript history cells.

use std::sync::Arc;

use ratatui::text::Line;

use crate::history_cell::{HistoryCell, PlainHistoryCell, history_cell_for};
use crate::vibe::event::V3DisplayEvent;

pub type TranscriptCells = Vec<Arc<dyn HistoryCell>>;

/// Builds the source-of-truth cell list used by scrollback replay and the transcript pager.
pub fn events_to_transcript_cells(events: &[V3DisplayEvent]) -> TranscriptCells {
    let mut cells = events
        .iter()
        .map(|event| Arc::<dyn HistoryCell>::from(history_cell_for(event)))
        .collect::<TranscriptCells>();
    if cells.is_empty() {
        cells.push(Arc::new(PlainHistoryCell::label(
            "transcript",
            "No transcript content available",
        )));
    }
    cells
}

/// Flattens cells at the current width for terminal history insertion or resize replay.
pub fn transcript_lines(cells: &[Arc<dyn HistoryCell>], width: u16) -> Vec<Line<'static>> {
    cells.iter().flat_map(|cell| cell.display_lines(width)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3_events_become_source_backed_transcript_cells() {
        let cells = events_to_transcript_cells(&[
            V3DisplayEvent::User { text: "hello".into(), attachments: Vec::new() },
            V3DisplayEvent::Assistant { markdown: "world".into() },
        ]);
        let rendered = transcript_lines(&cells, 40)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("hello"));
        assert!(rendered.contains("world"));
    }

    #[test]
    fn empty_transcript_has_explicit_placeholder() {
        let cells = events_to_transcript_cells(&[]);
        assert_eq!(cells.len(), 1);
        assert!(transcript_lines(&cells, 40)[0].to_string().contains("No transcript"));
    }
}
