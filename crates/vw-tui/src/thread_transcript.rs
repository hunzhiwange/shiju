//! 将 VibeWindow 展示事件渲染为记录历史单元。
//! Render VibeWindow display events into transcript history cells.

use std::sync::Arc;

use ratatui::text::Line;

use crate::history_cell::{HistoryCell, PlainHistoryCell, history_cell_for};
use crate::vibe::event::V3DisplayEvent;

pub type TranscriptCells = Vec<Arc<dyn HistoryCell>>;

/// 构建回滚区重放和记录分页器共同使用的权威单元列表。
/// Builds the source-of-truth cell list used by scrollback replay and the transcript pager.
pub fn events_to_transcript_cells(events: &[V3DisplayEvent]) -> TranscriptCells {
    let mut cells = events
        .iter()
        .map(|event| Arc::<dyn HistoryCell>::from(history_cell_for(event)))
        .collect::<TranscriptCells>();
    // 空记录也生成显式占位单元，避免调用方把“无内容”误判为未加载。
    if cells.is_empty() {
        cells.push(Arc::new(PlainHistoryCell::label(
            "transcript",
            "No transcript content available",
        )));
    }
    cells
}

/// 按当前宽度展平单元，供终端历史插入或尺寸变化后重放。
/// Flattens cells at the current width for terminal history insertion or resize replay.
pub fn transcript_lines(cells: &[Arc<dyn HistoryCell>], width: u16) -> Vec<Line<'static>> {
    cells.iter().flat_map(|cell| cell.display_lines(width)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // 用户和助手事件应成为可按宽度重新渲染的来源单元。
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
    // 空事件流必须显示明确占位内容，而非返回空单元列表。
    fn empty_transcript_has_explicit_placeholder() {
        let cells = events_to_transcript_cells(&[]);
        assert_eq!(cells.len(), 1);
        assert!(transcript_lines(&cells, 40)[0].to_string().contains("No transcript"));
    }
}
