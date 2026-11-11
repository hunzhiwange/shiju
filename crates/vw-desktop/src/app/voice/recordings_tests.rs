use super::*;
fn wav() -> Vec<u8> {
    crate::app::voice::capture::Recording { generation: 1, pcm: vec![500; 480] }.wav().unwrap()
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
    let large =
        crate::app::voice::capture::Recording { generation: 1, pcm: vec![500; 24_000 * 180] }
            .wav()
            .unwrap();
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
#[test]
fn voice_task_14_recordings_replay_drop_silences_callback() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    struct Stream(Arc<AtomicBool>);
    impl Drop for Stream {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = dropped.clone();
    let (tx, rx) = mpsc::channel();
    let audio =
        crate::app::voice::capture::Recording { generation: 1, pcm: vec![2000; 24_000 * 3] }
            .wav()
            .unwrap();
    let replay = super::super::recordings_playback::Replay::start_output(audio, move || {
        let (playback, output) = super::super::playback::fixture_stream(24_000, 1, Stream(marker));
        tx.send(output).unwrap();
        Ok(playback)
    })
    .unwrap();
    let mut output = rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut samples = [0f32; 480];
    loop {
        output.render(&mut samples);
        if samples.iter().any(|s| *s != 0.0) {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    drop(replay);
    assert!(dropped.load(Ordering::Acquire));
    output.render(&mut samples);
    assert!(samples.iter().all(|s| *s == 0.0));
}
