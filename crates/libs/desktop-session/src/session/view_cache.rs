//! Bounded LRU and session-open lifecycle, independent of render payloads.
use std::collections::{HashMap, VecDeque};
use std::mem;
use web_time::{Duration, Instant};
pub const SESSION_VIEW_CACHE_MAX_ENTRIES: usize = 3;
pub const SESSION_VIEW_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;
pub trait ViewSnapshot {
 fn estimated_bytes(&self) -> usize;
 fn content_signature(&self) -> u64;
}
/// 将快照插入缓存后的明确结果。
pub enum SessionViewCacheInsertResult<S> {
    Inserted { evicted_session_ids: Vec<String> },
    RejectedOverCapacity { snapshot: S, estimated_bytes: usize },
}

/// 热缓存发生容量回收时的原因。该枚举只描述容量条件，不携带会话或消息内容。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SessionViewCacheEvictionReason {
    #[default]
    None,
    EntryLimit,
    ByteLimit,
    EntryAndByteLimit,
    SnapshotOverCapacity,
}

impl SessionViewCacheEvictionReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::EntryLimit => "entry_limit",
            Self::ByteLimit => "byte_limit",
            Self::EntryAndByteLimit => "entry_and_byte_limit",
            Self::SnapshotOverCapacity => "snapshot_over_capacity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOpenUiCommit {
    HotRestore,
    PreparedHistory,
    SameContentRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOpenTraceSummary {
    source: &'static str,
    cache_hit: bool,
    message_count: usize,
    preparation_elapsed: Duration,
    main_thread_commit_elapsed: Duration,
    hot_restore_commits: u32,
    cold_commits: u32,
    same_content_refreshes: u32,
    changed_content_refreshes: u32,
    eviction_reason: SessionViewCacheEvictionReason,
}

impl SessionOpenTraceSummary {
    #[cfg(test)]
    pub fn source(&self) -> &'static str {
        self.source
    }

    #[cfg(test)]
    pub fn cache_hit(&self) -> bool {
        self.cache_hit
    }

    #[cfg(test)]
    pub fn ui_commit_count(&self) -> u32 {
        self.hot_restore_commits + self.cold_commits + self.changed_content_refreshes
    }

    #[cfg(test)]
    pub fn eviction_reason(&self) -> SessionViewCacheEvictionReason {
        self.eviction_reason
    }

    /// 输出一次会话打开汇总。字段只包含计数、耗时和固定枚举值。
    pub fn emit(&self) {
        let span = tracing::info_span!(
            "session_open",
            source = self.source,
            cache_hit = self.cache_hit,
            message_count = self.message_count,
            preparation_elapsed_ms = self.preparation_elapsed.as_millis() as u64,
            main_thread_commit_elapsed_ms = self.main_thread_commit_elapsed.as_millis() as u64,
            eviction_reason = self.eviction_reason.as_str(),
        );
        let _entered = span.enter();
        tracing::info!(
            target: "vw_desktop",
            hot_restore_commits = self.hot_restore_commits,
            cold_commits = self.cold_commits,
            same_content_refreshes = self.same_content_refreshes,
            changed_content_refreshes = self.changed_content_refreshes,
            "session open summary"
        );
    }
}

struct ActiveSessionOpen {
    session_id: String,
    generation: u64,
    started_at: Instant,
    message_count: usize,
    main_thread_commit_elapsed: Duration,
    hot_restore_commits: u32,
    cold_commits: u32,
    same_content_refreshes: u32,
    changed_content_refreshes: u32,
    eviction_reason: SessionViewCacheEvictionReason,
}

impl ActiveSessionOpen {
    fn record_commit(
        &mut self,
        kind: SessionOpenUiCommit,
        message_count: usize,
        commit_elapsed: Duration,
    ) {
        self.message_count = message_count;
        self.main_thread_commit_elapsed =
            self.main_thread_commit_elapsed.saturating_add(commit_elapsed);
        match kind {
            SessionOpenUiCommit::HotRestore => self.hot_restore_commits += 1,
            SessionOpenUiCommit::PreparedHistory if self.cold_commits == 0 => {
                self.cold_commits += 1;
            }
            SessionOpenUiCommit::PreparedHistory => self.changed_content_refreshes += 1,
            SessionOpenUiCommit::SameContentRefresh => self.same_content_refreshes += 1,
        }
    }

    fn summary(self) -> SessionOpenTraceSummary {
        let cache_hit = self.hot_restore_commits != 0;
        let source = if cache_hit {
            "view_cache"
        } else if self.cold_commits != 0
            && (self.same_content_refreshes != 0 || self.changed_content_refreshes != 0)
        {
            "ui_store_then_remote_history"
        } else {
            "remote_history"
        };
        SessionOpenTraceSummary {
            source,
            cache_hit,
            message_count: self.message_count,
            preparation_elapsed: self.started_at.elapsed(),
            main_thread_commit_elapsed: self.main_thread_commit_elapsed,
            hot_restore_commits: self.hot_restore_commits,
            cold_commits: self.cold_commits,
            same_content_refreshes: self.same_content_refreshes,
            changed_content_refreshes: self.changed_content_refreshes,
            eviction_reason: self.eviction_reason,
        }
    }
}

/// 确定性 LRU 会话视图缓存。
pub struct SessionViewCache<S> {
    entries: HashMap<String, S>,
    most_recent_last: VecDeque<String>,
    estimated_bytes: usize,
    active_open: Option<ActiveSessionOpen>,
    next_open_generation: u64,
    pub navigation_generation: u64,
    pending_eviction_reason: SessionViewCacheEvictionReason,
}

impl<S: ViewSnapshot> SessionViewCache<S> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            most_recent_last: VecDeque::new(),
            estimated_bytes: 0,
            active_open: None,
            next_open_generation: 0,
            navigation_generation: 0,
            pending_eviction_reason: SessionViewCacheEvictionReason::None,
        }
    }

    /// 为一次打开请求分配世代，防止晚到的后台结果覆盖当前会话。
    pub fn begin_open(&mut self, session_id: &str) -> u64 {
        self.next_open_generation = self.next_open_generation.saturating_add(1);
        let generation = self.next_open_generation;
        self.active_open = Some(ActiveSessionOpen {
            session_id: session_id.to_owned(),
            generation,
            started_at: Instant::now(),
            message_count: 0,
            main_thread_commit_elapsed: Duration::ZERO,
            hot_restore_commits: 0,
            cold_commits: 0,
            same_content_refreshes: 0,
            changed_content_refreshes: 0,
            eviction_reason: mem::take(&mut self.pending_eviction_reason),
        });
        generation
    }

    pub fn open_generation(&self) -> u64 {
        self.next_open_generation
    }

    pub fn is_current_open(&self, session_id: &str, generation: u64) -> bool {
        generation == 0
            || self.active_open.as_ref().is_some_and(|active_open| {
                active_open.session_id == session_id && active_open.generation == generation
            })
    }

    /// 记录一次主线程提交；`finalize` 时返回唯一的打开汇总事件。
    pub fn record_open_commit(
        &mut self,
        session_id: &str,
        generation: u64,
        kind: SessionOpenUiCommit,
        message_count: usize,
        commit_elapsed: Duration,
        finalize: bool,
    ) -> Option<SessionOpenTraceSummary> {
        if !self.is_current_open(session_id, generation) {
            return None;
        }
        let active_open = self.active_open.as_mut()?;
        active_open.record_commit(kind, message_count, commit_elapsed);
        if finalize { self.active_open.take().map(ActiveSessionOpen::summary) } else { None }
    }

    pub fn insert(
        &mut self,
        session_id: String,
        snapshot: S,
    ) -> SessionViewCacheInsertResult<S> {
        let snapshot_bytes = snapshot.estimated_bytes();
        if snapshot_bytes > SESSION_VIEW_CACHE_MAX_BYTES {
            self.pending_eviction_reason = SessionViewCacheEvictionReason::SnapshotOverCapacity;
            return SessionViewCacheInsertResult::RejectedOverCapacity {
                snapshot,
                estimated_bytes: snapshot_bytes,
            };
        }

        if let Some(previous) = self.entries.remove(&session_id) {
            self.estimated_bytes = self.estimated_bytes.saturating_sub(previous.estimated_bytes());
            self.most_recent_last.retain(|existing| existing != &session_id);
        }

        self.estimated_bytes = self.estimated_bytes.saturating_add(snapshot_bytes);
        self.most_recent_last.push_back(session_id.clone());
        self.entries.insert(session_id, snapshot);

        let mut evicted_session_ids = Vec::new();
        let mut evicted_for_entry_limit = false;
        let mut evicted_for_byte_limit = false;
        while self.entries.len() > SESSION_VIEW_CACHE_MAX_ENTRIES
            || self.estimated_bytes > SESSION_VIEW_CACHE_MAX_BYTES
        {
            evicted_for_entry_limit |= self.entries.len() > SESSION_VIEW_CACHE_MAX_ENTRIES;
            evicted_for_byte_limit |= self.estimated_bytes > SESSION_VIEW_CACHE_MAX_BYTES;
            let Some(oldest_session_id) = self.most_recent_last.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&oldest_session_id) {
                self.estimated_bytes =
                    self.estimated_bytes.saturating_sub(evicted.estimated_bytes());
                evicted_session_ids.push(oldest_session_id);
            }
        }

        self.pending_eviction_reason = if evicted_session_ids.is_empty() {
            SessionViewCacheEvictionReason::None
        } else {
            match (evicted_for_entry_limit, evicted_for_byte_limit) {
                (true, true) => SessionViewCacheEvictionReason::EntryAndByteLimit,
                (true, false) => SessionViewCacheEvictionReason::EntryLimit,
                (false, true) => SessionViewCacheEvictionReason::ByteLimit,
                (false, false) => SessionViewCacheEvictionReason::EntryLimit,
            }
        };

        SessionViewCacheInsertResult::Inserted { evicted_session_ids }
    }

    #[cfg(test)]
    pub fn get(&mut self, session_id: &str) -> Option<&S> {
        if !self.entries.contains_key(session_id) {
            return None;
        }
        self.most_recent_last.retain(|existing| existing != session_id);
        self.most_recent_last.push_back(session_id.to_owned());
        self.entries.get(session_id)
    }

    pub fn take(&mut self, session_id: &str) -> Option<S> {
        let snapshot = self.entries.remove(session_id)?;
        self.estimated_bytes = self.estimated_bytes.saturating_sub(snapshot.estimated_bytes());
        self.most_recent_last.retain(|existing| existing != session_id);
        Some(snapshot)
    }

    /// 仅在写入内容与热快照不同的时候使其失效，保留相同内容的工作集。
    pub fn invalidate_if_content_changed(
        &mut self,
        session_id: &str,
        content_signature: u64,
    ) -> bool {
        if self
            .entries
            .get(session_id)
            .is_none_or(|snapshot| snapshot.content_signature() == content_signature)
        {
            return false;
        }
        self.take(session_id).is_some()
    }

    pub fn remove(&mut self, session_id: &str) -> Option<S> {
        self.take(session_id)
    }

    #[cfg(test)]
    pub fn contains(&self, session_id: &str) -> bool {
        self.entries.contains_key(session_id)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<S: ViewSnapshot> Default for SessionViewCache<S> {
    fn default() -> Self {
        Self::new()
    }
}
