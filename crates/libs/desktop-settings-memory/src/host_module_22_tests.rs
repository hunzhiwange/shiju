//! Complete memory/storage page behavior migrated from the desktop host.
mod memory;
mod storage;
mod support;
#[cfg(not(target_arch = "wasm32"))]
mod ui;

#[test]
fn save_receipts_replace_only_the_owned_error() {
    let mut memory = crate::state::MemorySettingsState::default();
    let mut storage = crate::state::StorageSettingsState::default();
    crate::save_feedback::memory_saved(&mut memory, Some("memory failure".into()));
    crate::save_feedback::storage_saved(&mut storage, Some("storage failure".into()));
    assert_eq!(memory.save_error.as_deref(), Some("memory failure"));
    assert_eq!(storage.save_error.as_deref(), Some("storage failure"));
    crate::save_feedback::memory_saved(&mut memory, None);
    assert!(memory.save_error.is_none());
    assert_eq!(storage.save_error.as_deref(), Some("storage failure"));
    crate::save_feedback::storage_saved(&mut storage, None);
    assert!(storage.save_error.is_none());
}
