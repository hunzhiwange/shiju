/// A stable identity for a persisted or locally active chat entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChatEntryKey {
    PersistedId(String),
    PersistedSeq { session_id: String, seq_no: u64 },
    Active { generation: u64, local_item_id: u64 },
}

/// Monotonically increasing content revision for one stable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatEntryRevision(u64);

impl ChatEntryRevision {
    pub const INITIAL: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
