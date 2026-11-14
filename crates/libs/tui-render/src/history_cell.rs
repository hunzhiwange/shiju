//! 历史展示单元的共享渲染协议。

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

/// 复制版 Codex 转录中的单个展示单元。
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
