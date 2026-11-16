//! Terminal geometry policy shared by the main app and standalone screens.
//! 主应用与独立界面共享的终端几何策略。
//!
//! 普通按键和重绘复用缓存尺寸；仅在 resize、resume 或延迟复查期限到达时访问后端。

use std::io;
use std::time::Duration;
use std::time::Instant;

use ratatui::layout::Size;

use super::Tui;
use super::TuiEvent;
use crate::transcript_reflow::TRANSCRIPT_REFLOW_DEBOUNCE;

/// 跟踪延迟复查、当前事件绘制尺寸与一次性延后尺寸。
#[derive(Default)]
pub(super) struct ScreenSizePolicy {
    pending_recheck_at: Option<Instant>,
    pending_draw_size: Option<Size>,
    deferred_size: Option<Size>,
}

impl Tui {
    /// Resolve event geometry while avoiding backend queries on ordinary repaint frames.
    /// 解析事件对应几何，同时避免普通重绘帧查询终端后端。
    pub(crate) fn screen_size_for_event(&mut self, event: &TuiEvent) -> io::Result<Size> {
        if matches!(event, TuiEvent::Resize(_)) {
            self.schedule_screen_size_recheck(TRANSCRIPT_REFLOW_DEBOUNCE);
        } else if matches!(event, TuiEvent::Draw)
            && let Some(deadline) = self.screen_size.pending_recheck_at
        {
            let now = Instant::now();
            if now < deadline {
                self.frame_requester.schedule_frame_in(deadline - now);
            } else {
                return self.screen_size_for_event(&TuiEvent::Resume);
            }
        }

        let cached = self.terminal.last_known_screen_size;
        let size = match event {
            TuiEvent::Resize(size) => {
                self.screen_size.deferred_size = None;
                *size
            }
            TuiEvent::Resume => {
                self.screen_size.pending_recheck_at = None;
                self.screen_size.deferred_size = None;
                self.terminal.size()?
            }
            TuiEvent::Draw => self.screen_size.deferred_size.take().unwrap_or(cached),
            TuiEvent::Key(_) | TuiEvent::Paste(_) => cached,
        };
        self.screen_size.pending_draw_size =
            (!matches!(event, TuiEvent::Key(_) | TuiEvent::Paste(_))).then_some(size);
        Ok(size)
    }

    /// Schedule one backend sample after a resize, reflow, or external program.
    /// 在 resize、重排或外部程序结束后安排一次后端尺寸采样。
    pub(crate) fn schedule_screen_size_recheck(&mut self, delay: Duration) {
        self.screen_size.pending_recheck_at = Some(Instant::now() + delay);
        self.frame_requester.schedule_frame_in(delay);
    }

    /// 把已知尺寸留给下一次 Draw 使用，避免同一事件周期重复查询。
    pub(crate) fn defer_screen_size(&mut self, size: Size) {
        self.screen_size.deferred_size = Some(size);
    }

    /// 消费当前事件尺寸；没有缓存时查询后端并清空过期的延迟状态。
    pub(super) fn take_event_screen_size(&mut self) -> io::Result<Size> {
        if let Some(size) = self.screen_size.pending_draw_size.take() {
            return Ok(size);
        }
        let size = self.terminal.size()?;
        self.screen_size.pending_recheck_at = None;
        self.screen_size.deferred_size = None;
        Ok(size)
    }
}

#[cfg(test)]
// 尺寸缓存、延迟复查与事件选择测试独立存放。
#[path = "screen_size_tests.rs"]
mod tests;
