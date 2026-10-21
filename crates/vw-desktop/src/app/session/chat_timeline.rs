#![allow(dead_code)] // Wired into App by task 7; this task intentionally builds the pure model first.

use super::ChatMessage;
use std::collections::HashSet;

/// A stable identity for a persisted or locally active chat entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ChatEntryKey {
    PersistedId(String),
    PersistedSeq { session_id: String, seq_no: u64 },
    Active { generation: u64, local_item_id: u64 },
}

/// Monotonically increasing content revision for one stable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ChatEntryRevision(u64);

impl ChatEntryRevision {
    pub(crate) const INITIAL: Self = Self(0);

    pub(crate) fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// One loaded timeline row and its absolute persisted sequence, when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatTimelineEntry {
    pub(crate) key: ChatEntryKey,
    pub(crate) seq_no: Option<u64>,
    pub(crate) revision: ChatEntryRevision,
    pub(crate) message: ChatMessage,
}

impl ChatTimelineEntry {
    pub(crate) fn persisted(
        session_id: &str,
        seq_no: u64,
        message_id: Option<String>,
        revision: ChatEntryRevision,
        message: ChatMessage,
    ) -> Self {
        let key = message_id.map_or_else(
            || ChatEntryKey::PersistedSeq { session_id: session_id.to_owned(), seq_no },
            ChatEntryKey::PersistedId,
        );
        Self { key, seq_no: Some(seq_no), revision, message }
    }
}

/// A bounded response page. Cursors are opaque to the timeline.
#[derive(Debug, Clone)]
pub(crate) struct TimelinePage {
    pub(crate) entries: Vec<ChatTimelineEntry>,
    pub(crate) before_cursor: Option<String>,
    pub(crate) after_cursor: Option<String>,
    pub(crate) total_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageMergeOutcome {
    Applied { inserted: usize },
    NoProgress,
    RejectedStale,
}

/// Pure state for bounded history pages plus one mutable active entry overlay.
#[derive(Debug)]
pub(crate) struct PagedChatTimeline {
    session_id: String,
    generation: u64,
    entries: Vec<ChatTimelineEntry>,
    active_entry: Option<ChatTimelineEntry>,
    before_cursor: Option<String>,
    after_cursor: Option<String>,
    total_count: usize,
    in_flight_before: Option<String>,
    attempted_before: HashSet<String>,
    before_exhausted: bool,
}

impl PagedChatTimeline {
    pub(crate) fn new(session_id: impl Into<String>, generation: u64) -> Self {
        Self {
            session_id: session_id.into(),
            generation,
            entries: Vec::new(),
            active_entry: None,
            before_cursor: None,
            after_cursor: None,
            total_count: 0,
            in_flight_before: None,
            attempted_before: HashSet::new(),
            before_exhausted: false,
        }
    }

    pub(crate) fn entries(&self) -> &[ChatTimelineEntry] {
        &self.entries
    }

    pub(crate) fn active_entry(&self) -> Option<&ChatTimelineEntry> {
        self.active_entry.as_ref()
    }

    pub(crate) fn before_cursor(&self) -> Option<&str> {
        self.before_cursor.as_deref()
    }

    pub(crate) fn after_cursor(&self) -> Option<&str> {
        self.after_cursor.as_deref()
    }

    pub(crate) fn total_count(&self) -> usize {
        self.total_count
    }

    pub(crate) fn in_flight_before(&self) -> Option<&str> {
        self.in_flight_before.as_deref()
    }

    pub(crate) fn visible_entries(&self) -> Vec<&ChatTimelineEntry> {
        self.entries.iter().chain(self.active_entry.iter()).collect()
    }

    pub(crate) fn loaded_seq_range(&self) -> Option<(u64, u64)> {
        let mut seqs = self.entries.iter().filter_map(|entry| entry.seq_no);
        let first = seqs.next()?;
        Some(seqs.fold((first, first), |(min, max), seq| (min.min(seq), max.max(seq))))
    }

    pub(crate) fn merge_initial_page(&mut self, page: TimelinePage) -> PageMergeOutcome {
        let previous_len = self.entries.len();
        self.merge_entries(page.entries);
        let inserted = self.entries.len() - previous_len;
        self.total_count = page.total_count;
        self.after_cursor = page.after_cursor;

        if inserted == 0 && previous_len != 0 {
            return PageMergeOutcome::NoProgress;
        }
        self.before_exhausted = page.before_cursor.is_none();
        self.before_cursor = page.before_cursor;
        if inserted == 0 {
            PageMergeOutcome::NoProgress
        } else {
            PageMergeOutcome::Applied { inserted }
        }
    }

    pub(crate) fn begin_prepend(&mut self, cursor: &str) -> bool {
        if self.before_exhausted
            || self.in_flight_before.is_some()
            || self.before_cursor.as_deref() != Some(cursor)
            || self.attempted_before.contains(cursor)
        {
            return false;
        }
        self.attempted_before.insert(cursor.to_owned());
        self.in_flight_before = Some(cursor.to_owned());
        true
    }

    pub(crate) fn merge_prepend_page(
        &mut self,
        session_id: &str,
        generation: u64,
        requested_cursor: &str,
        page: TimelinePage,
    ) -> PageMergeOutcome {
        if session_id != self.session_id
            || generation != self.generation
            || self.in_flight_before.as_deref() != Some(requested_cursor)
        {
            return PageMergeOutcome::RejectedStale;
        }
        self.in_flight_before = None;
        let previous_len = self.entries.len();
        self.merge_entries(page.entries);
        let inserted = self.entries.len() - previous_len;
        self.total_count = page.total_count;

        let repeated_cursor = page.before_cursor.as_deref() == Some(requested_cursor);
        self.before_exhausted = inserted == 0 || repeated_cursor || page.before_cursor.is_none();
        self.before_cursor = page.before_cursor;
        if page.after_cursor.is_some() {
            self.after_cursor = page.after_cursor;
        }

        if inserted == 0 {
            PageMergeOutcome::NoProgress
        } else {
            PageMergeOutcome::Applied { inserted }
        }
    }

    pub(crate) fn is_before_exhausted(&self) -> bool {
        self.before_exhausted
    }

    pub(crate) fn set_active(&mut self, local_item_id: u64, message: ChatMessage) -> ChatEntryKey {
        let key = ChatEntryKey::Active { generation: self.generation, local_item_id };
        match self.active_entry.as_mut() {
            Some(entry) if entry.key == key => {
                entry.message = message;
                entry.revision = entry.revision.next();
            }
            _ => {
                self.active_entry = Some(ChatTimelineEntry {
                    key: key.clone(),
                    seq_no: None,
                    revision: ChatEntryRevision::INITIAL,
                    message,
                });
            }
        }
        key
    }

    pub(crate) fn complete_active(
        &mut self,
        seq_no: u64,
        message_id: Option<String>,
    ) -> Option<ChatTimelineEntry> {
        let active = self.active_entry.take()?;
        let completed = ChatTimelineEntry::persisted(
            &self.session_id,
            seq_no,
            message_id,
            active.revision,
            active.message,
        );
        self.merge_entries(vec![completed.clone()]);
        self.total_count = self.total_count.max(self.entries.len());
        Some(completed)
    }

    pub(crate) fn accepts_entry_result(
        &self,
        session_id: &str,
        generation: u64,
        key: &ChatEntryKey,
        revision: ChatEntryRevision,
    ) -> bool {
        session_id == self.session_id
            && generation == self.generation
            && self
                .entries
                .iter()
                .chain(self.active_entry.iter())
                .any(|entry| &entry.key == key && entry.revision == revision)
    }

    fn merge_entries(&mut self, incoming: Vec<ChatTimelineEntry>) {
        let mut existing: HashSet<_> = self.entries.iter().map(|entry| entry.key.clone()).collect();
        self.entries
            .extend(incoming.into_iter().filter(|entry| existing.insert(entry.key.clone())));
        self.entries.sort_by_key(|entry| entry.seq_no.unwrap_or(u64::MAX));
    }
}
