use super::*;
use std::path::Path;

#[test]
fn snapshots_match_only_identical_text() {
    let snapshot = FileSnapshot::from_text("hello");
    assert!(snapshot.matches_text("hello"));
    assert!(!snapshot.matches_text("hello!"));
    assert_eq!(snapshot.size_bytes, 5);
}

#[test]
fn cache_tracks_reads_and_invalidates_by_normalized_path() {
    let root = Path::new("/tmp/workspace");
    let mut cache = FileReadStateCache::new(4, 100);
    let normalized = cache.note_read(Some(root), "src/../file.txt", 7, false, None, None, None);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.total_bytes(), 7);
    assert_eq!(cache.get(Some(root), "file.txt").unwrap().path, normalized);
    assert!(cache.invalidate(Some(root), "file.txt").is_some());
    assert!(cache.is_empty());
}

#[test]
fn cache_evicts_oldest_when_limits_are_exceeded() {
    let mut cache = FileReadStateCache::new(1, 10);
    cache.note_read(None, "a.txt", 4, false, None, None, None);
    cache.note_read(None, "b.txt", 4, false, None, None, None);
    assert!(cache.get(None, "a.txt").is_none());
    assert!(cache.get(None, "b.txt").is_some());
}

#[test]
fn cache_replaces_entries_and_clear_resets_accounting() {
    let mut cache = FileReadStateCache::new(2, 20);
    cache.note_read(None, "same.txt", 8, false, None, None, None);
    cache.note_read(None, "same.txt", 3, true, Some(1), Some(2), None);

    let entry = cache.get(None, "same.txt").expect("replacement should remain cached");
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.total_bytes(), 3);
    assert!(entry.partial_view);
    assert_eq!(entry.offset, Some(1));
    assert_eq!(entry.limit, Some(2));

    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.total_bytes(), 0);
}

#[test]
fn cache_get_refreshes_lru_before_entry_limit_eviction() {
    let mut cache = FileReadStateCache::new(2, 100);
    cache.note_read(None, "a.txt", 1, false, None, None, None);
    cache.note_read(None, "b.txt", 1, false, None, None, None);
    assert!(cache.get(None, "a.txt").is_some());

    cache.note_read(None, "c.txt", 1, false, None, None, None);

    assert!(cache.get(None, "a.txt").is_some());
    assert!(cache.get(None, "b.txt").is_none());
    assert!(cache.get(None, "c.txt").is_some());
}

#[test]
fn cache_merge_obeys_byte_limit_and_preserves_newest_entry() {
    let mut source = FileReadStateCache::new(3, 100);
    source.note_read(None, "small.txt", 4, false, None, None, None);
    source.note_read(None, "large.txt", 8, false, None, None, None);

    let mut target = FileReadStateCache::new(3, 8);
    target.merge(&source);

    assert_eq!(target.total_bytes(), 8);
    assert!(target.get(None, "small.txt").is_none());
    assert!(target.get(None, "large.txt").is_some());
}

#[test]
fn normalized_path_handles_relative_components_without_a_root() {
    assert_eq!(
        FileReadStateCache::normalized_path(None, "missing-read-state/a/../b.txt"),
        Path::new("missing-read-state/b.txt")
    );
    assert_eq!(
        FileReadStateCache::normalized_path(None, "missing-read-state/a/../.."),
        Path::new(".")
    );
}
