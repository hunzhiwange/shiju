//! Local archive replay owns the native output on one cancellable thread.
use super::{
    playback::{Playback, PlaybackError},
    recordings::pcm,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub(crate) struct Replay {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    result: Arc<Mutex<Option<Result<(), String>>>>,
}
impl Replay {
    pub(crate) fn start(wav: Vec<u8>) -> Result<Self, String> {
        Self::start_output(wav, Playback::start)
    }
    pub(super) fn start_output<S: 'static>(
        wav: Vec<u8>,
        open: impl FnOnce() -> Result<Playback<S>, PlaybackError> + Send + 'static,
    ) -> Result<Self, String> {
        pcm(&wav)?;
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        let result = Arc::new(Mutex::new(None));
        let outcome = result.clone();
        let worker = std::thread::Builder::new()
            .name("voice-recording-replay".into())
            .spawn(move || {
                let play = || -> Result<(), String> {
                    if cancel.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    let mut playback = open().map_err(|_| "扬声器不可用。")?;
                    let started = Instant::now();
                    let mut submitted = 0u64;
                    for chunk in pcm(&wav)?.chunks(960) {
                        while submitted.saturating_sub(playback.played_ms() * 24) > 24_000 {
                            if cancel.load(Ordering::Acquire) {
                                return Ok(());
                            }
                            if playback.error().is_some()
                                || started.elapsed() > Duration::from_secs(190)
                            {
                                return Err("录音播放失败或超时。".into());
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        if cancel.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        playback.push_pcm(chunk).map_err(|_| "录音播放失败。")?;
                        submitted += (chunk.len() / 2) as u64;
                    }
                    while playback.pending() {
                        if cancel.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        if playback.error().is_some()
                            || started.elapsed() > Duration::from_secs(190)
                        {
                            return Err("录音播放失败或超时。".into());
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(())
                };
                *outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(play());
            })
            .map_err(|_| "无法启动录音播放。")?;
        Ok(Self { stop, worker: Some(worker), result })
    }
    pub(crate) fn result(&self) -> Option<Result<(), String>> {
        self.result.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}
impl Drop for Replay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
