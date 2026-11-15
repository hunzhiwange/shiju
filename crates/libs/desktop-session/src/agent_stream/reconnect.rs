use super::*;

pub(super) const GATEWAY_STREAM_INITIAL_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_millis(500);
pub(super) const GATEWAY_STREAM_MAX_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_secs(3 * 60 * 60);
pub(super) const PLAN_RUN_DELTA_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);
pub(super) const PLAN_RUN_DELTA_HARD_LIMIT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlanRunDeltaBatch {
    pub(super) session_id: String,
    pub(super) delta: String,
    pub(super) event_count: usize,
    pub(super) first_event_id: u64,
    pub(super) last_event_id: u64,
    pub(super) started_at: std::time::Duration,
    pub(super) char_count: usize,
    pub(super) cursor_event_id: Option<u64>,
}

impl PlanRunDeltaBatch {
    pub(super) fn new(
        session_id: &str,
        delta: &str,
        event_id: u64,
        started_at: std::time::Duration,
        event_complete: bool,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            delta: delta.to_string(),
            event_count: 1,
            first_event_id: event_id,
            last_event_id: event_id,
            started_at,
            char_count: delta.chars().count(),
            cursor_event_id: event_complete.then_some(event_id),
        }
    }

    pub(super) fn append_complete_event(&mut self, delta: &str, event_id: u64) {
        self.delta.push_str(delta);
        self.event_count = self.event_count.saturating_add(1);
        self.last_event_id = event_id;
        self.char_count = self.char_count.saturating_add(delta.chars().count());
        self.cursor_event_id = Some(event_id);
    }
}

#[derive(Debug, Default)]
pub(super) struct PlanRunDeltaBuffer {
    pub(super) pending: Option<PlanRunDeltaBatch>,
}

impl PlanRunDeltaBuffer {
    pub(super) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn push_delta<E>(
        &mut self,
        session_id: &str,
        delta: &str,
        event_id: u64,
        now: std::time::Duration,
        send: &mut impl FnMut(&PlanRunDeltaBatch) -> Result<(), E>,
    ) -> Result<Option<u64>, E> {
        let mut forwarded_cursor = None;
        let should_flush = self.pending.as_ref().is_some_and(|pending| {
            pending.session_id != session_id
                || now.saturating_sub(pending.started_at) >= PLAN_RUN_DELTA_FLUSH_INTERVAL
        });
        if should_flush {
            merge_forwarded_cursor(&mut forwarded_cursor, self.flush(send)?);
        }

        if self.pending.as_ref().is_some_and(|pending| {
            pending.delta.len().saturating_add(delta.len()) > PLAN_RUN_DELTA_HARD_LIMIT_BYTES
        }) {
            merge_forwarded_cursor(&mut forwarded_cursor, self.flush(send)?);
        }

        let mut remaining = delta;
        while remaining.len() > PLAN_RUN_DELTA_HARD_LIMIT_BYTES {
            let split_at = utf8_prefix_at_most(remaining, PLAN_RUN_DELTA_HARD_LIMIT_BYTES);
            let (chunk, rest) = remaining.split_at(split_at);
            self.pending = Some(PlanRunDeltaBatch::new(session_id, chunk, event_id, now, false));
            merge_forwarded_cursor(&mut forwarded_cursor, self.flush(send)?);
            remaining = rest;
        }

        match self.pending.as_mut() {
            Some(pending) => pending.append_complete_event(remaining, event_id),
            None => {
                self.pending =
                    Some(PlanRunDeltaBatch::new(session_id, remaining, event_id, now, true));
            }
        }

        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.delta.len() >= PLAN_RUN_DELTA_HARD_LIMIT_BYTES)
        {
            merge_forwarded_cursor(&mut forwarded_cursor, self.flush(send)?);
        }

        Ok(forwarded_cursor)
    }

    pub(super) fn flush<E>(
        &mut self,
        send: &mut impl FnMut(&PlanRunDeltaBatch) -> Result<(), E>,
    ) -> Result<Option<u64>, E> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(None);
        };
        send(pending)?;
        let cursor_event_id = pending.cursor_event_id;
        self.pending = None;
        Ok(cursor_event_id)
    }
}

pub(super) fn merge_forwarded_cursor(current: &mut Option<u64>, next: Option<u64>) {
    if let Some(next) = next {
        *current = Some(current.map_or(next, |current| current.max(next)));
    }
}

pub(super) fn utf8_prefix_at_most(value: &str, max_bytes: usize) -> usize {
    let mut split_at = value.len().min(max_bytes);
    while !value.is_char_boundary(split_at) {
        split_at = split_at.saturating_sub(1);
    }
    split_at
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GatewayStreamStopReason {
    Backpressure,
    Cancelled,
    PostToolRoundHandoff,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum GatewayStreamNext {
    Terminal,
    Retry(std::time::Duration),
    Cancelled,
    PostToolRoundHandoff,
    Failed(String),
}

#[derive(Debug)]
pub(super) struct GatewayReconnectState {
    pub(super) request_id: String,
    pub(super) last_event_id: Option<u64>,
    pub(super) next_delay: std::time::Duration,
}

impl GatewayReconnectState {
    pub(super) fn new(request_id: String) -> Self {
        Self { request_id, last_event_id: None, next_delay: GATEWAY_STREAM_INITIAL_RETRY_DELAY }
    }

    pub(super) fn resume(&self) -> gateway_chat_stream::GatewayStreamResume {
        gateway_chat_stream::GatewayStreamResume {
            request_id: self.request_id.clone(),
            last_event_id: self.last_event_id,
        }
    }

    pub(super) fn finish_attempt(
        &mut self,
        result: Result<gateway_chat_stream::GatewayStreamAttemptResult, String>,
        saw_new_event: bool,
        last_forwarded_event_id: Option<u64>,
        stop_reason: Option<GatewayStreamStopReason>,
    ) -> GatewayStreamNext {
        if let Some(event_id) = last_forwarded_event_id {
            self.record_forwarded_event(event_id);
        }
        if saw_new_event {
            self.next_delay = GATEWAY_STREAM_INITIAL_RETRY_DELAY;
        }

        match stop_reason {
            Some(GatewayStreamStopReason::Cancelled) => return GatewayStreamNext::Cancelled,
            Some(GatewayStreamStopReason::PostToolRoundHandoff) => {
                return GatewayStreamNext::PostToolRoundHandoff;
            }
            Some(GatewayStreamStopReason::Backpressure) => {
                return GatewayStreamNext::Retry(self.take_retry_delay());
            }
            None => {}
        }

        match result {
            Ok(gateway_chat_stream::GatewayStreamAttemptResult::Terminated) => {
                GatewayStreamNext::Terminal
            }
            Ok(gateway_chat_stream::GatewayStreamAttemptResult::CallerStopped) => {
                GatewayStreamNext::Cancelled
            }
            Ok(gateway_chat_stream::GatewayStreamAttemptResult::Incomplete) => {
                self.last_event_id.get_or_insert(0);
                GatewayStreamNext::Retry(self.take_retry_delay())
            }
            Err(error) if gateway_stream_error_is_retryable(&error) => {
                GatewayStreamNext::Retry(self.take_retry_delay())
            }
            Err(error) => GatewayStreamNext::Failed(error),
        }
    }

    pub(super) fn record_forwarded_event(&mut self, event_id: u64) {
        self.last_event_id =
            Some(self.last_event_id.map_or(event_id, |current| current.max(event_id)));
    }

    pub(super) fn take_retry_delay(&mut self) -> std::time::Duration {
        let delay = self.next_delay;
        self.next_delay = self.next_delay.saturating_mul(2).min(GATEWAY_STREAM_MAX_RETRY_DELAY);
        delay
    }
}

pub(super) fn gateway_stream_error_is_retryable(error: &str) -> bool {
    if error.starts_with("请求内容过大") {
        return false;
    }

    let lower = error.to_ascii_lowercase();
    for prefix in ["gateway stream failed: ", "gateway request failed: "] {
        let Some(status_text) = lower.strip_prefix(prefix) else {
            continue;
        };
        let Some(status) = status_text.get(..3).and_then(|value| value.parse::<u16>().ok()) else {
            continue;
        };
        return status == 408 || status == 429 || status >= 500;
    }

    true
}
