//! Single input-device lease, bounded PCM queue and generation-scoped recording.
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 24_000;
pub const FRAME_SAMPLES: usize = 480;
pub const MAX_SECONDS: u64 = 180;
const QUEUE_FRAMES: usize = 50;
const RUNNING: u8 = 0;
const FINISH: u8 = 1;
const CANCEL: u8 = 2;
const DEVICE_ERROR: u8 = 3;
const BACKPRESSURE: u8 = 4;
const STREAM_ERROR: u8 = 5;
const PERMISSION_DENIED: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    Busy,
    Permission,
    NoDevice,
    DeviceRemoved,
    Device,
    UnsupportedFormat,
    Cancelled,
    Timeout,
    Limit,
    Backpressure,
    Silent,
}
impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceRemoved => f.write_str("input device disappeared; reselect a microphone"),
            _ => write!(f, "audio capture: {self:?}"),
        }
    }
}
impl std::error::Error for CaptureError {}
type Result<T> = std::result::Result<T, CaptureError>;

#[derive(Debug, Clone)]
pub struct InputDevice {
    /// Native host-local ID; never a display name or a remote host's identifier.
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn input_devices() -> Result<Vec<InputDevice>> {
    super::capture_native::input_devices()
}

pub struct Recording {
    pub generation: u64,
    pub pcm: Vec<i16>,
}
impl Recording {
    /// Mono 24 kHz signed PCM16 little-endian RIFF/WAVE, bounded to 180 seconds.
    pub fn wav(&self) -> Result<Vec<u8>> {
        if self.pcm.len() > SAMPLE_RATE as usize * MAX_SECONDS as usize {
            return Err(CaptureError::Limit);
        }
        let size = (self.pcm.len() * 2) as u32;
        let mut out = Vec::with_capacity(44 + size as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + size).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
        out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        out.extend_from_slice(b"\x02\0\x10\0data");
        out.extend_from_slice(&size.to_le_bytes());
        for sample in &self.pcm {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(out)
    }
}
impl std::fmt::Debug for Recording {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recording")
            .field("generation", &self.generation)
            .field("sample_count", &self.pcm.len())
            .finish()
    }
}

#[derive(Clone)]
pub(super) struct CaptureSink {
    pub(super) generation: u64,
    sender: SyncSender<Frame>,
    epoch: Arc<AtomicU64>,
    state: Arc<AtomicU8>,
}
struct Frame {
    generation: u64,
    epoch: u64,
    samples: [i16; FRAME_SAMPLES],
    len: usize,
}
impl CaptureSink {
    #[cfg(test)]
    pub(super) fn send(&self, samples: [i16; FRAME_SAMPLES], len: usize) {
        self.send_epoch(samples, len, self.epoch.load(Ordering::Acquire));
    }
    fn send_epoch(&self, samples: [i16; FRAME_SAMPLES], len: usize, epoch: u64) {
        if self.state.load(Ordering::Acquire) != RUNNING {
            return;
        }
        if self.sender.try_send(Frame { generation: self.generation, epoch, samples, len }).is_err()
        {
            self.fail(BACKPRESSURE);
        }
    }
    fn fail(&self, reason: u8) {
        let _ = self.state.compare_exchange(RUNNING, reason, Ordering::AcqRel, Ordering::Acquire);
    }
    pub(super) fn error(&self, error: CaptureError) {
        self.fail(match error {
            CaptureError::DeviceRemoved => DEVICE_ERROR,
            CaptureError::Permission => PERMISSION_DENIED,
            _ => STREAM_ERROR,
        });
    }
}

/// Only the physical device boundary is replaceable. Stream destruction releases the device.
pub(super) trait CaptureBackend: Send + 'static {
    type Stream;
    fn open(self, device_id: Option<String>, sink: CaptureSink) -> Result<Self::Stream>;
}
pub(super) fn device_lease() -> Arc<AtomicBool> {
    static LEASE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    LEASE.get_or_init(|| Arc::new(AtomicBool::new(false))).clone()
}

/// Bounded live PCM, using the same exclusive microphone lease as dictation.
/// No complete recording is retained for a conversation.
pub struct LiveCapture {
    pub capture: Capture,
    frames: Receiver<LiveFrame>,
}
impl LiveCapture {
    pub fn start() -> Result<Self> {
        Self::start_device(None)
    }
    pub fn start_device(device: Option<String>) -> Result<Self> {
        use super::permissions::{SystemPermissions, VoiceCapability, VoicePermissions};
        VoicePermissions::new(SystemPermissions)
            .start(VoiceCapability::Recording, || {
                Self::open_device(super::capture_native::NativeCapture, device_lease(), device)
                    .map_err(anyhow::Error::from)
            })
            .map_err(|error| error.downcast::<CaptureError>().unwrap_or(CaptureError::Permission))
    }
    #[cfg(test)]
    pub(super) fn start_backend<B: CaptureBackend>(
        backend: B,
        lease: Arc<AtomicBool>,
    ) -> Result<Self> {
        Self::open_device(backend, lease, None)
    }
    fn open_device<B: CaptureBackend>(
        backend: B,
        lease: Arc<AtomicBool>,
        device: Option<String>,
    ) -> Result<Self> {
        let (sender, frames) = mpsc::sync_channel(QUEUE_FRAMES);
        let capture =
            Capture::open_backend(backend, device, Duration::from_secs(1800), lease, Some(sender))?;
        Ok(Self { capture, frames })
    }
    pub fn into_parts(self) -> (Capture, LiveFrames) {
        {
            let epoch = self.capture.epoch.clone();
            (self.capture, LiveFrames { frames: self.frames, epoch })
        }
    }
}
struct LiveFrame {
    epoch: u64,
    samples: Vec<i16>,
}
pub struct LiveFrames {
    frames: Receiver<LiveFrame>,
    epoch: Arc<AtomicU64>,
}
impl LiveFrames {
    pub fn next_frame(&self) -> Option<Vec<i16>> {
        while let Ok(frame) = self.frames.try_recv() {
            if frame.epoch == self.epoch.load(Ordering::Acquire) {
                return Some(frame.samples);
            }
        }
        None
    }
    /// Invalidate both the callback queue and the consumer queue, including in-flight frames.
    pub fn clear(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        while self.frames.try_recv().is_ok() {}
    }
}

struct Lease(Arc<AtomicBool>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Latest accepted PCM frame; polling does not consume audio or reset the sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeSummary {
    pub sequence: u64,
    pub peak: f32,
}

pub struct Capture {
    pub generation: u64,
    state: Arc<AtomicU8>,
    result: Receiver<Result<Recording>>,
    epoch: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
    volume: Arc<AtomicU64>,
}
impl Capture {
    /// One coherent latest-frame snapshot, with fixed storage and no audio consumption.
    /// Sequence zero means no accepted samples (or capture has ended).
    pub fn volume_summary(&self) -> VolumeSummary {
        if self.state.load(Ordering::Acquire) != RUNNING {
            return VolumeSummary { sequence: 0, peak: 0.0 };
        }
        let packed = self.volume.load(Ordering::Acquire);
        VolumeSummary { sequence: packed >> 16, peak: (packed & 0xffff) as f32 / 32768.0 }
    }
    /// None resolves the default device once; no fallback after selection/disconnection.
    /// Recording and realtime chat must both acquire through this entry point.
    pub fn start(device_id: Option<String>) -> Result<Self> {
        Self::start_with_limit(device_id, Duration::from_secs(MAX_SECONDS))
    }
    /// Uses the same permission gate and device lease, with a caller's shorter bound.
    pub fn start_with_limit(device_id: Option<String>, limit: Duration) -> Result<Self> {
        use super::permissions::{SystemPermissions, VoiceCapability, VoicePermissions};

        VoicePermissions::new(SystemPermissions)
            .start(VoiceCapability::Recording, || {
                Self::start_backend(
                    super::capture_native::NativeCapture,
                    device_id,
                    limit,
                    device_lease(),
                )
                .map_err(anyhow::Error::from)
            })
            .map_err(|error| error.downcast::<CaptureError>().unwrap_or(CaptureError::Permission))
    }
    pub(super) fn start_backend<B: CaptureBackend>(
        backend: B,
        device_id: Option<String>,
        limit: Duration,
        lease: Arc<AtomicBool>,
    ) -> Result<Self> {
        if limit.is_zero() || limit > Duration::from_secs(MAX_SECONDS) {
            return Err(CaptureError::Limit);
        }
        Self::open_backend(backend, device_id, limit, lease, None)
    }
    fn open_backend<B: CaptureBackend>(
        backend: B,
        device_id: Option<String>,
        limit: Duration,
        lease: Arc<AtomicBool>,
        live: Option<SyncSender<LiveFrame>>,
    ) -> Result<Self> {
        lease
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| CaptureError::Busy)?;
        let lease = Lease(lease);
        static GENERATION: AtomicU64 = AtomicU64::new(1);
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (sender, frames) = mpsc::sync_channel(QUEUE_FRAMES);
        let epoch = Arc::new(AtomicU64::new(1));
        let sink = CaptureSink { generation, sender, state: state.clone(), epoch: epoch.clone() };
        let (ready_tx, ready) = mpsc::sync_channel(1);
        let (result_tx, result) = mpsc::sync_channel(1);
        let worker_state = state.clone();
        let worker_epoch = epoch.clone();
        let volume = Arc::new(AtomicU64::new(0));
        let worker_volume = volume.clone();
        let worker = thread::Builder::new()
            .name("voice-capture".into())
            .spawn(move || {
                let lease_guard = lease;
                let stream = match backend.open(device_id, sink) {
                    Ok(stream) => stream,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let outcome = collect(
                    generation,
                    limit,
                    &worker_state,
                    frames,
                    live,
                    &worker_volume,
                    &worker_epoch,
                );
                worker_state.store(CANCEL, Ordering::Release);
                drop(stream);
                drop(lease_guard);
                let _ = result_tx.send(outcome);
            })
            .map_err(|_| CaptureError::Device)?;
        if let Err(error) = ready.recv().unwrap_or(Err(CaptureError::Device)) {
            let _ = worker.join();
            return Err(error);
        }
        Ok(Self { generation, state, result, worker: Some(worker), epoch, volume })
    }
    pub fn finish(mut self) -> Result<Recording> {
        let _ = self.state.compare_exchange(RUNNING, FINISH, Ordering::AcqRel, Ordering::Acquire);
        self.wait()
    }
    pub fn cancel(mut self) -> Result<Recording> {
        self.state.store(CANCEL, Ordering::Release);
        self.wait()
    }
    pub fn try_result(&mut self) -> Option<Result<Recording>> {
        self.result.try_recv().ok()
    }
    fn wait(&mut self) -> Result<Recording> {
        let outcome = self.result.recv().unwrap_or(Err(CaptureError::Device));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        outcome
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.state.store(CANCEL, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn collect(
    generation: u64,
    limit: Duration,
    state: &AtomicU8,
    frames: Receiver<Frame>,
    live: Option<SyncSender<LiveFrame>>,
    volume: &AtomicU64,
    epoch: &AtomicU64,
) -> Result<Recording> {
    let deadline = Instant::now() + limit;
    let max_samples = (limit.as_secs_f64() * SAMPLE_RATE as f64) as usize;
    let mut pcm = Vec::with_capacity(if live.is_some() { 0 } else { max_samples });
    loop {
        match state.load(Ordering::Acquire) {
            FINISH => {
                while let Ok(frame) = frames.try_recv() {
                    append(&mut pcm, frame, generation, max_samples)?;
                }
                if pcm.iter().all(|sample| sample.unsigned_abs() <= 8) {
                    return Err(CaptureError::Silent);
                }
                return Ok(Recording { generation, pcm });
            }
            CANCEL => return Err(CaptureError::Cancelled),
            DEVICE_ERROR => return Err(CaptureError::DeviceRemoved),
            STREAM_ERROR => return Err(CaptureError::Device),
            PERMISSION_DENIED => return Err(CaptureError::Permission),
            BACKPRESSURE => return Err(CaptureError::Backpressure),
            _ => {}
        }
        if Instant::now() >= deadline {
            return Err(CaptureError::Timeout);
        }
        match frames.recv_timeout(Duration::from_millis(5)) {
            Ok(frame) => {
                if frame.generation != generation {
                    continue;
                }
                if frame.len > FRAME_SAMPLES {
                    return Err(CaptureError::Limit);
                }
                let peak =
                    frame.samples[..frame.len].iter().map(|sample| sample.unsigned_abs()).max();
                let current_epoch = frame.epoch == epoch.load(Ordering::Acquire);
                if let Some(sender) = &live {
                    sender
                        .try_send(LiveFrame {
                            epoch: frame.epoch,
                            samples: frame.samples[..frame.len].to_vec(),
                        })
                        .map_err(|_| CaptureError::Backpressure)?;
                } else {
                    append(&mut pcm, frame, generation, max_samples)?;
                }
                if let Some(peak) = peak.filter(|_| current_epoch) {
                    // Single writer; one atomic packs a 48-bit frame sequence and PCM16 peak.
                    // Even the longest live capture cannot exhaust this sequence space.
                    let sequence = (volume.load(Ordering::Relaxed) >> 16) + 1;
                    volume.store((sequence << 16) | u64::from(peak), Ordering::Release);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(CaptureError::DeviceRemoved),
        }
    }
}
fn append(pcm: &mut Vec<i16>, frame: Frame, generation: u64, max_samples: usize) -> Result<()> {
    if frame.generation != generation {
        return Ok(());
    }
    if frame.len > FRAME_SAMPLES || pcm.len() + frame.len >= max_samples {
        return Err(CaptureError::Limit);
    }
    pcm.extend_from_slice(&frame.samples[..frame.len]);
    Ok(())
}

/// Area resampler: average channels, then integrate sample intervals into 24 kHz bins.
/// Handles non-integer rates across callbacks with fixed storage, no IO or allocation.
pub(super) struct PcmConverter {
    sink: CaptureSink,
    epoch: u64,
    rate: u32,
    remaining: u32,
    sum: f64,
    samples: [i16; FRAME_SAMPLES],
    len: usize,
}
impl PcmConverter {
    pub(super) fn new(sink: CaptureSink, rate: u32) -> Result<Self> {
        if !(8_000..=192_000).contains(&rate) {
            return Err(CaptureError::UnsupportedFormat);
        }
        let epoch = sink.epoch.load(Ordering::Acquire);
        Ok(Self {
            sink,
            epoch,
            rate,
            remaining: rate,
            sum: 0.0,
            samples: [0; FRAME_SAMPLES],
            len: 0,
        })
    }
    pub(super) fn mono(&mut self, sample: f32) {
        let epoch = self.sink.epoch.load(Ordering::Acquire);
        if epoch != self.epoch {
            self.epoch = epoch;
            self.remaining = self.rate;
            self.sum = 0.0;
            self.len = 0;
        }
        let sample = if sample.is_finite() { sample.clamp(-1.0, 1.0) } else { 0.0 };
        let mut width = SAMPLE_RATE;
        while width > 0 {
            let take = width.min(self.remaining);
            self.sum += sample as f64 * take as f64;
            width -= take;
            self.remaining -= take;
            if self.remaining == 0 {
                self.samples[self.len] =
                    (self.sum / self.rate as f64 * i16::MAX as f64).round() as i16;
                self.len += 1;
                self.remaining = self.rate;
                self.sum = 0.0;
                if self.len == FRAME_SAMPLES {
                    self.sink.send_epoch(self.samples, self.len, self.epoch);
                    self.len = 0;
                }
            }
        }
    }
    pub(super) fn flush(&mut self) {
        if self.len > 0 {
            self.sink.send_epoch(self.samples, self.len, self.epoch);
            self.len = 0;
        }
    }
}
