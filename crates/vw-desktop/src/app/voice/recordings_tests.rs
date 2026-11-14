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
