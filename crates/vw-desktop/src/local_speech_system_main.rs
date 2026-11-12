//! Explicit system-boundary acceptance. No text/audio payload is logged.
#[cfg(target_os = "macos")]
mod system {
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    use vw_desktop::app::voice::{local::*, permissions::PermissionStatus};

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *const std::ffi::c_void;
        fn CFRunLoopRunInMode(
            mode: *const std::ffi::c_void,
            seconds: f64,
            return_after_source: bool,
        ) -> i32;
    }
    pub fn run() -> std::result::Result<(), String> {
        let arguments = std::env::args().skip(1).collect::<Vec<_>>();
        let transcribe = match arguments.as_slice() {
            [] => false,
            [flag] if flag == "--transcribe" => true,
            _ => return Err("Usage: local-speech-system [--transcribe]".into()),
        };
        let cancel = Cancellation::default();
        let token = cancel.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let result = validate(transcribe, token);
            let _ = tx.send(result);
        });
        let deadline = Instant::now() + Duration::from_secs(if transcribe { 220 } else { 150 });
        let result = loop {
            if let Ok(result) = rx.try_recv() {
                break result;
            }
            if Instant::now() > deadline {
                cancel.cancel();
                return Err(
                    "System speech probe timed out; no successful recognition was recorded".into(),
                );
            }
            // The actual Iced desktop already runs this main loop. Tests need
            // an explicit main-thread driver for Apple's synthesis service.
            unsafe {
                CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.02, true);
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        worker.join().map_err(|_| "Speech worker panicked")?;
        result
    }
    fn validate(transcribe: bool, cancel: Cancellation) -> std::result::Result<(), String> {
        let state = capabilities(Some("zh-CN")).map_err(|error| error.to_string())?;
        println!(
            "language={} on_device={} available={} authorization={:?} voices={}",
            state.language,
            state.supports_on_device,
            state.available,
            state.permission,
            state.voices.len()
        );
        let voice = state
            .voices
            .iter()
            .find(|voice| voice.language == "zh-CN")
            .ok_or("No installed Apple zh-CN system voice; install one in System Settings")?;
        let audio = synthesize("你好，这是本机离线语音测试。", Some(&voice.id), &cancel)
            .map_err(|error| error.to_string())?;
        if audio.pcm.is_empty() || !audio.pcm.iter().any(|sample| sample.unsigned_abs() > 16) {
            return Err("Local synthesis returned silent/empty audio".into());
        }
        println!("synthesis_rate={} synthesis_samples={}", SAMPLE_RATE, audio.pcm.len());
        let start = Instant::now();
        let token = Cancellation::default();
        let child = token.clone();
        let selected = voice.id.clone();
        let worker = std::thread::spawn(move || {
            synthesize(&"这是一个取消测试。".repeat(150), Some(&selected), &child)
        });
        std::thread::sleep(Duration::from_millis(50));
        token.cancel();
        let cancelled = worker.join().map_err(|_| "Cancellation worker panicked")?;
        if !matches!(cancelled, Err(LocalSpeechError::Cancelled))
            || start.elapsed() > Duration::from_secs(3)
        {
            return Err("Synthesis cancellation did not release within 3 seconds".into());
        }
        println!("synthesis_cancel_ms={}", start.elapsed().as_millis());
        if transcribe {
            let result = transcribe_pcm(&audio.pcm, Some("zh-CN"), &cancel)
                .map_err(|error| error.to_string())?;
            if !(result.text.contains("语音") || result.text.contains("测试")) {
                return Err("Chinese fixture was not recognized as expected".into());
            }
            println!("offline_chinese_transcription=passed");
        } else if state.permission != PermissionStatus::Granted {
            println!("offline_chinese_transcription=not_run_speech_permission_required");
        } else {
            println!("offline_chinese_transcription=not_run_use_transcribe_flag");
        }
        Ok(())
    }
}
fn main() {
    #[cfg(target_os = "macos")]
    if let Err(error) = system::run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Local system speech probe requires macOS");
        std::process::exit(2);
    }
}
