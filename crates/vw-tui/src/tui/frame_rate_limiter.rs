//! Limits how frequently frame draw notifications may be emitted.
//!
//! Widgets sometimes call `FrameRequester::schedule_frame()` more frequently than a user can
//! perceive. This limiter clamps draw notifications to a maximum of 120 FPS to avoid wasted work.
//!
//! This is intentionally a small, pure helper so it can be unit-tested in isolation and used by
//! the async frame scheduler without adding complexity to the app/event loop.
//!
//! 控件可能比用户可感知速度更频繁地请求绘制。本纯辅助器把通知限制在最高 120 FPS，
//! 供异步调度器复用且不把限速状态耦合到主事件循环。

use std::time::Duration;
use std::time::Instant;

/// A 120 FPS minimum frame interval (≈8.33ms).
/// 120 FPS 对应的最小帧间隔（约 8.33 毫秒）。
pub(super) const MIN_FRAME_INTERVAL: Duration = Duration::from_nanos(8_333_334);

/// Remembers the most recent emitted draw, allowing deadlines to be clamped forward.
/// 记录最近一次实际发出的绘制时间，以便把过早期限向后钳制。
#[derive(Debug, Default)]
pub(super) struct FrameRateLimiter {
    last_emitted_at: Option<Instant>,
}

impl FrameRateLimiter {
    /// Returns `requested`, clamped forward if it would exceed the maximum frame rate.
    /// 返回请求时刻；若会超过最大帧率则向后钳制到允许的最早时刻。
    pub(super) fn clamp_deadline(&self, requested: Instant) -> Instant {
        let Some(last_emitted_at) = self.last_emitted_at else {
            return requested;
        };
        let min_allowed = last_emitted_at
            .checked_add(MIN_FRAME_INTERVAL)
            .unwrap_or(last_emitted_at);
        requested.max(min_allowed)
    }

    /// Records that a draw notification was emitted at `emitted_at`.
    /// 记录绘制通知实际发出的时刻。
    pub(super) fn mark_emitted(&mut self, emitted_at: Instant) {
        self.last_emitted_at = Some(emitted_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_does_not_clamp() {
        let t0 = Instant::now();
        let limiter = FrameRateLimiter::default();
        assert_eq!(limiter.clamp_deadline(t0), t0);
    }

    #[test]
    fn clamps_to_min_interval_since_last_emit() {
        let t0 = Instant::now();
        let mut limiter = FrameRateLimiter::default();

        assert_eq!(limiter.clamp_deadline(t0), t0);
        limiter.mark_emitted(t0);

        let too_soon = t0 + Duration::from_millis(1);
        assert_eq!(limiter.clamp_deadline(too_soon), t0 + MIN_FRAME_INTERVAL);
    }
}
