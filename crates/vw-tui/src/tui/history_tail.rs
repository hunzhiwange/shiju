//! Update visible terminal-history tails without discarding terminal scrollback.
//! 在不丢弃终端回滚缓冲的前提下替换当前可见历史尾部。

use super::Tui;
use crate::custom_terminal::Terminal as CustomTerminal;
use crate::insert_history::HistoryLineWrapPolicy;
use crate::insert_history::InsertHistoryMode;
use crate::insert_history::insert_history_hyperlink_lines_with_mode_and_wrap_policy;
use crate::insert_history::wrap_history_hyperlink_lines;
use crate::terminal_hyperlinks::HyperlinkLine;
use ratatui::backend::Backend;
use ratatui::layout::Position;
use std::io;
use std::io::Write;

impl Tui {
    #[cfg(test)]
    pub(crate) fn pending_history_lines_for_test(&self) -> Vec<HyperlinkLine> {
        self.pending_history_lines
            .iter()
            .flat_map(|batch| batch.lines.iter().cloned())
            .collect()
    }

    /// 先提交待写历史，再尝试原地替换可见尾部；成功时安排一帧重绘。
    pub(crate) fn replace_visible_history_tail(
        &mut self,
        previous_lines: &[HyperlinkLine],
        replacement: &[HyperlinkLine],
        wrap_policy: HistoryLineWrapPolicy,
    ) -> io::Result<bool> {
        let screen_size = self.terminal.last_known_screen_size;
        Self::flush_pending_history_lines(
            &mut self.terminal,
            &mut self.pending_history_lines,
            self.scrollback,
            screen_size,
        )?;
        let mode = self.scrollback.history_insertion_mode(wrap_policy);
        let replaced = replace_visible_terminal_history_tail(
            &mut self.terminal,
            previous_lines,
            replacement,
            mode,
            wrap_policy,
        )?;
        if replaced {
            self.frame_requester().schedule_frame();
        }
        Ok(replaced)
    }
}

/// 仅当旧尾部完整位于当前 viewport 上方时回退并替换，无法安全定位则返回 `false`。
fn replace_visible_terminal_history_tail<B>(
    terminal: &mut CustomTerminal<B>,
    previous_lines: &[HyperlinkLine],
    replacement: &[HyperlinkLine],
    mode: InsertHistoryMode,
    wrap_policy: HistoryLineWrapPolicy,
) -> io::Result<bool>
where
    B: Backend<Error = io::Error> + Write,
{
    let screen_size = terminal.last_known_screen_size;
    let mut viewport = terminal.viewport_area;
    let wrap_width = usize::from(viewport.width.max(/*other*/ 1));
    // 按当前终端宽度重新计算旧内容占用行数，避免宽字符或换行策略导致错位。
    let (_, previous_rows) = wrap_history_hyperlink_lines(previous_lines, wrap_width, wrap_policy);
    let Ok(previous_rows) = u16::try_from(previous_rows) else {
        return Ok(false);
    };
    if previous_rows == 0 || previous_rows > viewport.top() {
        return Ok(false);
    }

    viewport.y -= previous_rows;
    terminal.clear_after_position(Position::new(/*x*/ 0, viewport.y))?;
    terminal.set_viewport_area(viewport);
    terminal.invalidate_viewport();
    insert_history_hyperlink_lines_with_mode_and_wrap_policy(
        terminal,
        replacement,
        mode,
        wrap_policy,
        screen_size,
    )?;
    Ok(true)
}

#[cfg(test)]
// 可见尾部边界与回滚保留行为测试独立存放。
#[path = "history_tail_tests.rs"]
mod tests;
