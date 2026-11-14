use super::*;
fn wav() -> Vec<u8> {
    wav_samples(480)
}
#[test]
fn voice_task_14_recordings_evict_oldest_after_twenty() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("recordings")).unwrap();
    store.save(true, &wav()).unwrap();
    let oldest = store.list().unwrap()[0].id.clone();
    for _ in 0..20 {
        store.save(true, &wav()).unwrap();
    }
    assert_eq!(
        store.list().unwrap().len(),
        20,
        "only the most recent twenty recordings may remain"
    );
    assert!(!store.list().unwrap().iter().any(|e| e.id == oldest));
}
#[test]
fn voice_task_14_recordings_disabled_writes_no_audio() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("recordings")).unwrap();
    store.save(false, &wav()).unwrap();
    assert!(store.list().unwrap().is_empty(), "disabled retention must never write audio");
}
#[test]
fn voice_task_14_recordings_restart_delete_and_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("archive");
    let first;
    {
        let store = Store::open(&root).unwrap();
        store.save(true, &wav()).unwrap();
        first = store.list().unwrap()[0].id.clone();
        assert_eq!(store.read(&first).unwrap(), wav());
    }
    std::fs::write(root.join(".voice-interrupted"), b"partial").unwrap();
    let store = Store::open(&root).unwrap();
    assert!(!root.join(".voice-interrupted").exists());
    assert_eq!(store.list().unwrap()[0].id, first);
    std::fs::write(root.join(&first), b"broken").unwrap();
    assert!(store.read(&first).unwrap_err().contains("损坏"));
    store.delete(&first).unwrap();
    assert!(store.read(&first).is_err());
    store.save(true, &wav()).unwrap();
    store.clear().unwrap();
    assert!(store.list().unwrap().is_empty());
}
#[test]
fn voice_task_14_recordings_rejects_links_traversal_and_partial_audio() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("archive");
    let store = Store::open(&root).unwrap();
    assert!(store.save(true, b"RIFF partial").is_err());
    assert!(store.read("../escape").is_err());
    assert!(store.delete("../escape").is_err());
    let outside = dir.path().join("outside.wav");
    std::fs::write(&outside, wav()).unwrap();
    let id = "11111111111111111111111111111111.wav";
    symlink(&outside, root.join(id)).unwrap();
    assert!(store.read(id).is_err());
    store.delete(id).unwrap();
    assert!(outside.exists());
    assert_eq!(std::fs::metadata(&root).unwrap().permissions().mode() & 0o777, 0o700);
    store.save(true, &wav()).unwrap();
    let entry = store.list().unwrap().remove(0);
    assert_eq!(std::fs::metadata(root.join(entry.id)).unwrap().permissions().mode() & 0o777, 0o600);
    drop(store);
    let link = dir.path().join("linked");
    symlink(&root, &link).unwrap();
    assert!(Store::open(&link).is_err());
}
#[test]
fn voice_task_14_recordings_total_byte_budget_and_oldest_eviction() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("archive")).unwrap();
    let large = wav_samples(24_000 * 180);
    store.save(true, &large).unwrap();
    let first = store.list().unwrap()[0].id.clone();
    for _ in 0..8 {
        store.save(true, &large).unwrap();
    }
    let entries = store.list().unwrap();
    assert!(entries.len() < 20);
    assert!(entries.iter().map(|e| e.bytes).sum::<u64>() <= MAX_TOTAL);
    assert!(!entries.iter().any(|e| e.id == first));
}
#[test]
fn voice_task_14_recordings_held_directory_survives_replacement() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("archive");
    let store = Store::open(&root).unwrap();
    let held = dir.path().join("moved");
    std::fs::rename(&root, &held).unwrap();
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    symlink(&outside, &root).unwrap();
    store.save(true, &wav()).unwrap();
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&held).unwrap().count(), 1);
}
// Same mono 24 kHz PCM16 WAV shape as desktop Capture::Recording.
fn wav_samples(count: usize) -> Vec<u8> {
    let size = (count * 2) as u32;
    let mut out = Vec::with_capacity(44 + size as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + size).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    out.extend_from_slice(&24_000u32.to_le_bytes());
    out.extend_from_slice(&48_000u32.to_le_bytes());
    out.extend_from_slice(b"\x02\0\x10\0data");
    out.extend_from_slice(&size.to_le_bytes());
    for _ in 0..count {
        out.extend_from_slice(&500i16.to_le_bytes());
    }
    out
}
