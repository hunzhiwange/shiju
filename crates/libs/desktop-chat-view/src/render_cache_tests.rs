use desktop_session::timeline_identity::{ChatEntryKey, ChatEntryRevision};

#[test]
fn revision_render_reuses_unchanged_rows() {
    let key = ChatEntryKey::PersistedId("message-1".to_string());
    let cache_key = super::RevisionRenderKey::new(
        key.clone(),
        ChatEntryRevision::INITIAL,
        720.0,
        false,
        false,
    );

    assert!(cache_key.matches_key(&super::RevisionRenderKey::new(key.clone(), ChatEntryRevision::INITIAL, 719.0, false, false)));
}

#[test]
fn revision_render_invalidates_changed_entry_only() {
    let key = ChatEntryKey::PersistedId("message-1".to_string());
    let other = ChatEntryKey::PersistedId("message-2".to_string());
    let cache_key = super::RevisionRenderKey::new(
        key.clone(),
        ChatEntryRevision::INITIAL,
        720.0,
        false,
        false,
    );

    assert!(!cache_key.matches_key(&super::RevisionRenderKey::new(key.clone(), ChatEntryRevision::INITIAL.next(), 720.0, false, false)));
    assert!(!cache_key.matches_key(&super::RevisionRenderKey::new(other.clone(), ChatEntryRevision::INITIAL, 720.0, false, false)));
}

#[test]
fn revision_render_invalidates_theme_and_width() {
    let key = ChatEntryKey::PersistedId("message-1".to_string());
    let cache_key = super::RevisionRenderKey::new(
        key.clone(),
        ChatEntryRevision::INITIAL,
        720.0,
        false,
        false,
    );

    assert!(!cache_key.matches_key(&super::RevisionRenderKey::new(key.clone(), ChatEntryRevision::INITIAL, 800.0, false, false)));
    assert!(!cache_key.matches_key(&super::RevisionRenderKey::new(key.clone(), ChatEntryRevision::INITIAL, 720.0, true, false)));
}
