//! Test-only counters for chat hot-path workload baselines.

use std::cell::Cell;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChatPerformanceSnapshot {
    pub initial_load_messages: usize,
    pub initial_load_bytes: usize,
    pub chat_message_clone_bytes: usize,
    pub full_content_hashes: usize,
    pub full_content_hash_bytes: usize,
    pub full_content_parses: usize,
    pub full_content_parse_bytes: usize,
    pub height_index_rebuilds: usize,
    pub height_point_updates: usize,
    pub mounted_rows: usize,
    pub full_session_saves: usize,
    pub runtime_borrows: usize,
    pub runtime_clones: usize,
}

thread_local! {
    static COUNTERS: Cell<ChatPerformanceSnapshot> = const {
        Cell::new(ChatPerformanceSnapshot {
            initial_load_messages: 0,
            initial_load_bytes: 0,
            chat_message_clone_bytes: 0,
            full_content_hashes: 0,
            full_content_hash_bytes: 0,
            full_content_parses: 0,
            full_content_parse_bytes: 0,
            height_index_rebuilds: 0,
            height_point_updates: 0,
            mounted_rows: 0,
            full_session_saves: 0,
            runtime_borrows: 0,
            runtime_clones: 0,
        })
    };
}

fn update(f: impl FnOnce(&mut ChatPerformanceSnapshot)) {
    COUNTERS.with(|counters| {
        let mut current = counters.get();
        f(&mut current);
        counters.set(current);
    });
}

pub(crate) fn reset() {
    COUNTERS.with(|counters| counters.set(ChatPerformanceSnapshot::default()));
}

pub(crate) fn snapshot() -> ChatPerformanceSnapshot {
    COUNTERS.with(Cell::get)
}

pub(crate) fn record_initial_load(messages: usize, bytes: usize) {
    update(|snapshot| {
        snapshot.initial_load_messages += messages;
        snapshot.initial_load_bytes += bytes;
    });
}

pub(crate) fn record_chat_message_clone(bytes: usize) {
    update(|snapshot| snapshot.chat_message_clone_bytes += bytes);
}

pub(crate) fn record_full_content_hash(bytes: usize) {
    update(|snapshot| {
        snapshot.full_content_hashes += 1;
        snapshot.full_content_hash_bytes += bytes;
    });
}

pub(crate) fn record_full_content_parse(bytes: usize) {
    update(|snapshot| {
        snapshot.full_content_parses += 1;
        snapshot.full_content_parse_bytes += bytes;
    });
}

pub(crate) fn record_height_index_rebuild() {
    update(|snapshot| snapshot.height_index_rebuilds += 1);
}

pub(crate) fn record_height_point_update() {
    update(|snapshot| snapshot.height_point_updates += 1);
}

pub(crate) fn record_mounted_rows(rows: usize) {
    update(|snapshot| snapshot.mounted_rows += rows);
}

pub(crate) fn record_full_session_save() {
    update(|snapshot| snapshot.full_session_saves += 1);
}

pub(crate) fn record_runtime_borrow() {
    update(|snapshot| snapshot.runtime_borrows += 1);
}

pub(crate) fn record_runtime_clone() {
    update(|snapshot| snapshot.runtime_clones += 1);
}
