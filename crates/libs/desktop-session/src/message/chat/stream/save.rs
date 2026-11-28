//! 流式持久化队列、修订和重试状态。
#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;

#[derive(Debug, Clone, Default)]
pub struct StreamSaveState {
    pub flush_scheduled: bool,
    pub revision: u64,
    pub acked_revision: u64,
    pub failures: u8,
    pub seq_base: u64,
    pub generation: u64,
    pub expected_updated_ms: u64,
    pub dirty_records: Vec<session_ui_types::SessionMessageRecord>,
    pub in_flight: Option<(u64, u64, u64)>,
    history_conflict: bool,
    confirmed_identity_revision: u64,
}

pub const SAVE_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
pub const MAX_FAILURES: u8 = 3;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome { Ignored, Settled, Save, Retry, Stopped }
impl StreamSaveState {
    pub fn schedule(&mut self, is_plan_run: bool) -> Option<std::time::Duration> {
        if is_plan_run || self.flush_scheduled || self.failures >= MAX_FAILURES { return None; }
        self.flush_scheduled = true;
        Some(SAVE_FLUSH_INTERVAL * (1 << self.failures))
    }
    pub fn mark(&mut self, record: session_ui_types::SessionMessageRecord, base_updated_ms: u64, generation: Option<u64>) {
        if self.expected_updated_ms == 0 { self.expected_updated_ms = base_updated_ms; }
        if let Some(generation) = generation { self.generation = generation; }
        self.revision = self.revision.saturating_add(1);
        if let Some(existing) = self.dirty_records.iter_mut().find(|dirty| dirty.seq_no == record.seq_no) { *existing = record; }
        else { self.dirty_records.push(record); }
    }

    /// Only gateway-confirmed IDs may repair provisional records at the same sequence.
    pub fn confirm_terminal_ids(
        &mut self,
        generation: u64,
        chat: &[session_ui_types::ChatMessage],
        ids: &[Option<String>],
        parent_id: Option<&str>,
        assistant_id: Option<&str>,
    ) {
        use session_ui_types::ChatRole;

        if self.generation != generation {
            return;
        }
        let mut repaired = false;
        for record in &mut self.dirty_records {
            if record.message_id.is_some() {
                continue;
            }
            let confirmed_id = match record.message.role {
                ChatRole::User => parent_id,
                ChatRole::Assistant => assistant_id,
                _ => None,
            };
            let Some(confirmed_id) = confirmed_id else { continue };
            let Some(index) = record.seq_no.checked_sub(self.seq_base)
                .and_then(|index| usize::try_from(index).ok())
            else { continue };
            let Some(message) = chat.get(index) else { continue };
            if ids.get(index).and_then(Option::as_deref) != Some(confirmed_id)
                || message.role != record.message.role
                || !message.content.starts_with(&record.message.content)
                || (message.role == ChatRole::User && message.content != record.message.content)
            {
                continue;
            }
            record.message_id = Some(confirmed_id.to_owned());
            record.message = message.clone();
            repaired = true;
        }
        if repaired {
            self.revision = self.revision.saturating_add(1);
            self.confirmed_identity_revision = self.revision;
            if self.history_conflict {
                self.history_conflict = false;
                self.failures = 0;
            }
        }
    }
    pub fn prepare(&mut self, session_id: &str, is_plan_run: bool, now: u64) -> Option<(u64, u64, session_ui_types::SessionMessageDelta)> {
        if is_plan_run || self.in_flight.is_some() || self.failures >= MAX_FAILURES || self.dirty_records.is_empty() { return None; }
        let updated_ms = now.max(self.expected_updated_ms.saturating_add(1));
        let delta = session_ui_types::SessionMessageDelta { session_id: session_id.into(), expected_updated_ms: self.expected_updated_ms, updated_ms, upserts: self.dirty_records.clone(), truncate_after: None };
        self.in_flight = Some((self.generation, self.revision, updated_ms));
        Some((self.generation, self.revision, delta))
    }
    /// Missing persisted rows are recovered only from the caller's real bounded history.
    #[allow(clippy::too_many_arguments)]
    pub fn acknowledge(&mut self, generation: u64, revision: u64, updated_ms: u64, result: &Result<(), SessionMessageDeltaSaveError>, base_updated_ms: u64, request_generation: Option<u64>, mut repair: impl FnMut(usize) -> Option<(session_ui_types::ChatMessage, Option<String>)>) -> (SaveOutcome, bool) {
        if self.in_flight != Some((generation, revision, updated_ms)) { return (SaveOutcome::Ignored, false); }
        self.in_flight = None;
        match result {
            Ok(()) => {
                self.failures = 0; self.expected_updated_ms = updated_ms;
                self.history_conflict = false;
                self.acked_revision = self.acked_revision.max(revision);
                if self.revision == revision && self.generation == generation {
                    self.dirty_records.clear(); (SaveOutcome::Settled, false)
                } else { (SaveOutcome::Save, false) }
            }
            Err(error) => {
                self.failures = self.failures.saturating_add(1);
                self.history_conflict = false;
                let mut recoverable = true;
                let mut progress = false;
                let mut repaired = false;
                match error {
                    SessionMessageDeltaSaveError::Stale { stored_updated_ms, stored_message_count } => {
                        progress = self.expected_updated_ms != *stored_updated_ms;
                        self.expected_updated_ms = *stored_updated_ms;
                        if let Some(end) = self.dirty_records.iter().map(|record| record.seq_no).max() {
                            for seq_no in *stored_message_count as u64..=end {
                                if self.dirty_records.iter().any(|record| record.seq_no == seq_no) { continue; }
                                let Some(index) = seq_no.checked_sub(self.seq_base).and_then(|index| usize::try_from(index).ok()) else { recoverable = false; break; };
                                let Some((message, message_id)) = repair(index) else { recoverable = false; break; };
                                self.mark(session_ui_types::SessionMessageRecord { seq_no, message_id, message }, base_updated_ms, request_generation);
                                progress = true; repaired = true;
                            }
                        }
                    }
                    SessionMessageDeltaSaveError::HistoryConflict(_) => {
                        progress = self.confirmed_identity_revision > revision;
                        recoverable = progress;
                        self.history_conflict = !recoverable;
                    }
                    SessionMessageDeltaSaveError::Rejected(_) => recoverable = false,
                    SessionMessageDeltaSaveError::Other(_) => {}
                }
                if recoverable && progress { self.failures = 0; }
                if !recoverable || self.failures >= MAX_FAILURES {
                    self.failures = MAX_FAILURES; self.flush_scheduled = false;
                    (SaveOutcome::Stopped, repaired)
                } else { (SaveOutcome::Retry, repaired) }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMessageDeltaSaveError {
    Stale { stored_updated_ms: u64, stored_message_count: usize },
    HistoryConflict(String),
    Rejected(String),
    Other(String),
}

impl std::fmt::Display for SessionMessageDeltaSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale { stored_updated_ms, .. } => {
                write!(formatter, "stale session message delta at {stored_updated_ms}")
            }
            Self::HistoryConflict(error) | Self::Rejected(error) | Self::Other(error) => {
                formatter.write_str(error)
            }
        }
    }
}

impl StreamSaveState {
    /// Reuse only an exact pending append prefix; replacements take a fresh snapshot.
    pub fn snapshot(&mut self, source: &session_ui_types::ChatMessage, seq_no: u64, previous_content_len: Option<usize>) -> (session_ui_types::ChatMessage, bool) {
        let previous = previous_content_len.filter(|len| source.content.is_char_boundary(*len)).and_then(|len| {
            self.dirty_records.iter_mut().find(|record| record.seq_no == seq_no && record.message.content.len() == len).map(|record| std::mem::take(&mut record.message.content))
        });
        if let Some(mut content) = previous {
            content.push_str(&source.content[content.len()..]);
            (session_ui_types::ChatMessage { content, role: source.role, model: source.model.clone(), think_timing: source.think_timing.clone(), processing_timing: source.processing_timing.clone() }, false)
        } else { (source.clone(), true) }
    }
}
