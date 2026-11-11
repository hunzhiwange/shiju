use super::playback::{PlaybackError, fixture};
fn pcm(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}
#[test]
fn voice_task_08_playback_stop_silences_old_generation() {
    let (mut player, mut callback) = fixture(24000, 1);
    player.push_pcm(&pcm(&[16384; 20])).unwrap();
    let mut data = [0.0f32; 4];
    callback.render(&mut data);
    assert!(data.iter().all(|v| *v == 0.5));
    player.stop();
    callback.render(&mut data);
    assert_eq!(data, [0.0; 4]);
    assert_eq!(player.push_pcm(&pcm(&[1])), Err(PlaybackError::Stopped));
    let (mut next, mut next_callback) = fixture(24000, 1);
    next.push_pcm(&pcm(&[8192])).unwrap();
    callback.render(&mut data);
    assert_eq!(data, [0.0; 4]);
    next_callback.render(&mut data);
    assert_eq!(data, [0.25, 0.0, 0.0, 0.0]);
}
#[test]
fn voice_task_08_playback_underrun_outputs_silence() {
    let (mut player, mut callback) = fixture(48000, 2);
    player.push_pcm(&pcm(&[16384, -16384])).unwrap();
    let mut data = [1.0f32; 12];
    callback.render(&mut data);
    assert_eq!(data, [0.5, 0.5, 0.5, 0.5, -0.5, -0.5, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0]);
    callback.render(&mut data);
    assert_eq!(data, [0.0; 12]);
}
#[test]
fn voice_task_08_playback_limits_and_drop() {
    let (mut player, mut callback) = fixture(24000, 1);
    for _ in 0..10 {
        player.push_pcm(&vec![1; 9600]).unwrap();
    }
    assert_eq!(player.push_pcm(&[1, 0]), Err(PlaybackError::Overflow));
    let mut data = [1.0f32; 8];
    callback.render(&mut data);
    assert_eq!(data, [0.0; 8]);
    let (mut player, mut callback) = fixture(24000, 1);
    player.push_pcm(&pcm(&[16384])).unwrap();
    drop(player);
    callback.render(&mut data);
    assert_eq!(data, [0.0; 8]);
}
#[test]
fn voice_task_08_playback_format_error_invalidates_callback() {
    let (mut player, mut callback) = fixture(24000, 1);
    player.push_pcm(&pcm(&[16384; 8])).unwrap();
    assert_eq!(player.push_pcm(&[1]), Err(PlaybackError::Format));
    let mut data = [1.0f32; 4];
    callback.render(&mut data);
    assert_eq!(data, [0.0; 4]);
    assert_eq!(player.error(), Some(PlaybackError::Format));
}
#[test]
fn voice_task_08_playback_sine_preserves_order_across_packets() {
    let samples: Vec<i16> = (0..240)
        .map(|i| ((i as f32 * std::f32::consts::TAU / 24.0).sin() * 16000.0) as i16)
        .collect();
    let (mut player, mut callback) = fixture(24000, 2);
    for packet in samples.chunks(17) {
        player.push_pcm(&pcm(packet)).unwrap();
    }
    let mut result = Vec::new();
    for _ in 0..24 {
        let mut data = [0.0f32; 20];
        callback.render(&mut data);
        result.extend(data);
    }
    for (frame, expected) in result.chunks_exact(2).zip(samples) {
        assert_eq!(frame, [expected as f32 / 32768.0; 2]);
    }
}

#[test]
fn voice_task_08_playback_stop_and_drop_release_stream() {
    struct Stream(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl Drop for Stream {
        fn drop(&mut self) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let released = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (mut player, _) = super::playback::fixture_stream(24000, 1, Stream(released.clone()));
    player.stop();
    assert_eq!(released.load(std::sync::atomic::Ordering::SeqCst), 1);
    drop(player);
    assert_eq!(released.load(std::sync::atomic::Ordering::SeqCst), 1);
    let (player, _) = super::playback::fixture_stream(24000, 1, Stream(released.clone()));
    drop(player);
    assert_eq!(released.load(std::sync::atomic::Ordering::SeqCst), 2);
}
#[test]
fn voice_task_08_playback_device_error_silences_callback() {
    let (mut player, mut callback) = fixture(44100, 2);
    player.push_pcm(&pcm(&[16384; 100])).unwrap();
    let mut data = [0.0f32; 2];
    callback.render(&mut data);
    assert_eq!(data, [0.5; 2]);
    callback.fail(PlaybackError::Device);
    callback.render(&mut data);
    assert_eq!(data, [0.0; 2]);
    assert_eq!(player.error(), Some(PlaybackError::Device));
    assert_eq!(player.push_pcm(&[0, 0]), Err(PlaybackError::Device));
}
