use super::daily_memory::State;

#[test]
fn daily_editor_defers_creation_until_save() {
    let mut state = State::default();
    // Creating a draft is synchronous and has no client or filesystem capability.
    state.create_draft("2026-11-14.md".parse().unwrap());
    assert_eq!(state.filename.as_ref().unwrap().as_str(), "2026-11-14.md");
    assert!(state.loaded);
    assert!(state.editor.text().trim().is_empty());
    // Invalidation must drop the draft when the application context changes.
    state.invalidate();
    assert!(state.filename.is_none());
    assert!(!state.loaded);
}
