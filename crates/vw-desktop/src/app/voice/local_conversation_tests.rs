use super::*;
#[test]
fn voice_task_11_local_reads_only_final_prose() {
    assert_eq!(speech_text("<think>内部思考</think>你好。\n```rust\nsecret();\n```"), "你好。");
}
#[test]
fn voice_task_11_local_does_not_read_inline_code() {
    assert_eq!(speech_text("请查看 `secret_token()` 后继续。"), "请查看  后继续。");
}
#[test]
fn voice_task_11_local_playback_paces_pcm_and_cancels_within_bound() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let cancel = Cancellation::default();
    let done = Arc::new(AtomicBool::new(false));
    let finished = done.clone();
    let (playback, mut output) = super::super::playback::fixture(24_000, 1);
    let token = cancel.clone();
    let reader = std::thread::spawn(move || {
        let mut samples = [0i16; 240];
        let mut heard = false;
        for _ in 0..100 {
            output.render(&mut samples);
            heard |= samples.iter().any(|v| *v != 0);
            if heard {
                token.cancel();
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(30));
        output.render(&mut samples);
        assert!(samples.iter().all(|s| *s == 0));
        finished.store(true, Ordering::SeqCst);
        heard
    });
    let begin = Instant::now();
    let result = play(&vec![1000; 24_000 * 3], &cancel, || Ok(playback));
    assert!(result.is_err());
    assert!(begin.elapsed() < Duration::from_secs(1));
    assert!(reader.join().unwrap());
    assert!(done.load(Ordering::SeqCst));
}
#[test]
fn voice_task_11_local_playback_drains_more_than_two_second_capacity() {
    let (playback, mut output) = super::super::playback::fixture(24_000, 1);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let halt = stop.clone();
    let reader = std::thread::spawn(move || {
        let mut heard = 0;
        while !halt.load(std::sync::atomic::Ordering::SeqCst) {
            let mut data = [0i16; 240];
            output.render(&mut data);
            heard += data.iter().filter(|s| **s != 0).count();
            std::thread::sleep(Duration::from_millis(1));
        }
        heard
    });
    let result = play(&vec![1000; 72_000], &Cancellation::default(), || Ok(playback));
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(result, Ok(()));
    assert_eq!(reader.join().unwrap(), 72_000);
}
#[test]
fn voice_task_11_local_long_prose_has_explicit_bounded_prefix() {
    let prose = "中文回复".repeat(1000);
    let spoken = spoken_prefix(&prose);
    assert_eq!(spoken.chars().count(), local::MAX_TEXT_CHARS);
    assert!(spoken.ends_with("其余内容请查看聊天。"));
}
#[test]
fn voice_task_11_local_voice_preference_roundtrips_without_cloud_config() {
    use super::super::preferences::Preferences;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("voice.json");
    let preferences =
        Preferences { local_voice: Some("com.apple.voice.fixture".into()), ..Default::default() };
    preferences.write(&path).unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), preferences);
    let invalid = Preferences { local_voice: Some(String::new()), ..Default::default() };
    assert!(invalid.write(&path).is_err());
    assert_eq!(Preferences::read(&path).unwrap(), preferences);
}
