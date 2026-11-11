use super::capture::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct AudioDevice {
    sink: Arc<Mutex<Option<CaptureSink>>>,
    released: Arc<AtomicBool>,
}
struct Stream(Arc<AtomicBool>);
impl Drop for Stream {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl CaptureBackend for AudioDevice {
    type Stream = Stream;
    fn open(self, _: Option<String>, sink: CaptureSink) -> Result<Stream, CaptureError> {
        *self.sink.lock().unwrap() = Some(sink);
        Ok(Stream(self.released))
    }
}
fn device() -> AudioDevice {
    AudioDevice { sink: Arc::new(Mutex::new(None)), released: Arc::new(AtomicBool::new(false)) }
}
fn start(device: &AudioDevice, lease: Arc<AtomicBool>, limit: Duration) -> Capture {
    Capture::start_backend(device.clone(), None, limit, lease).unwrap()
}
fn sink(device: &AudioDevice) -> CaptureSink {
    device.sink.lock().unwrap().as_ref().unwrap().clone()
}
#[test]
fn voice_task_04_capture_cancel_discards_old_generation() {
    let lease = Arc::new(AtomicBool::new(false));
    let first = device();
    let capture = start(&first, lease.clone(), Duration::from_secs(1));
    let old = sink(&first);
    old.send([123; FRAME_SAMPLES], FRAME_SAMPLES);
    assert_eq!(capture.cancel().unwrap_err(), CaptureError::Cancelled);
    assert!(first.released.load(Ordering::Acquire));
    let second = device();
    let capture = start(&second, lease, Duration::from_secs(1));
    assert_ne!(capture.generation, old.generation);
    old.send([999; FRAME_SAMPLES], FRAME_SAMPLES);
    let mut stale = sink(&second);
    stale.generation = old.generation;
    stale.send([999; FRAME_SAMPLES], FRAME_SAMPLES);
    sink(&second).send([456; FRAME_SAMPLES], FRAME_SAMPLES);
    assert_eq!(capture.finish().unwrap().pcm, vec![456; FRAME_SAMPLES]);
}
#[test]
fn voice_task_04_capture_device_removed_releases_stream() {
    let lease = Arc::new(AtomicBool::new(false));
    let device = device();
    let mut capture = start(&device, lease.clone(), Duration::from_secs(1));
    sink(&device).send([123; FRAME_SAMPLES], FRAME_SAMPLES);
    sink(&device).error(CaptureError::DeviceRemoved);
    let deadline = Instant::now() + Duration::from_secs(1);
    let outcome = loop {
        if let Some(result) = capture.try_result() {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(outcome.unwrap_err(), CaptureError::DeviceRemoved);
    assert!(device.released.load(Ordering::Acquire));
    assert!(!lease.load(Ordering::Acquire));
}

fn await_result(capture: &mut Capture) -> Result<Recording, CaptureError> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(result) = capture.try_result() {
            return result;
        }
        assert!(Instant::now() < deadline, "capture must terminate autonomously");
        std::thread::yield_now();
    }
}
#[test]
fn voice_task_04_timeout_and_drop_release_shared_lease() {
    let lease = Arc::new(AtomicBool::new(false));
    let first = device();
    let mut capture = start(&first, lease.clone(), Duration::from_millis(20));
    assert_eq!(
        Capture::start_backend(first.clone(), None, Duration::from_secs(1), lease.clone()).err(),
        Some(CaptureError::Busy)
    );
    assert_eq!(await_result(&mut capture).unwrap_err(), CaptureError::Timeout);
    assert!(first.released.load(Ordering::Acquire));
    assert!(!lease.load(Ordering::Acquire));
    let second = device();
    drop(start(&second, lease.clone(), Duration::from_secs(1)));
    assert!(second.released.load(Ordering::Acquire));
    assert!(!lease.load(Ordering::Acquire));
}
#[test]
fn voice_task_04_silence_and_sample_limit_return_explicit_errors() {
    let first = device();
    let capture = start(&first, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
    sink(&first).send([0; FRAME_SAMPLES], FRAME_SAMPLES);
    assert_eq!(capture.finish().unwrap_err(), CaptureError::Silent);
    let second = device();
    let mut capture = start(&second, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
    for _ in 0..50 {
        sink(&second).send([321; FRAME_SAMPLES], FRAME_SAMPLES);
    }
    assert_eq!(await_result(&mut capture).unwrap_err(), CaptureError::Limit);
    assert!(second.released.load(Ordering::Acquire));
}
struct Flood;
impl CaptureBackend for Flood {
    type Stream = ();
    fn open(self, _: Option<String>, sink: CaptureSink) -> Result<(), CaptureError> {
        for _ in 0..51 {
            sink.send([321; FRAME_SAMPLES], FRAME_SAMPLES);
        }
        Ok(())
    }
}
#[test]
fn voice_task_04_full_queue_fails_without_partial_success() {
    let mut capture = Capture::start_backend(
        Flood,
        None,
        Duration::from_secs(1),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert_eq!(await_result(&mut capture).unwrap_err(), CaptureError::Backpressure);
}
#[test]
fn voice_task_04_resampling_and_wav_preserve_contract() {
    for rate in [8_000, 24_000, 44_100, 48_000, 96_000] {
        let device = device();
        let capture = start(&device, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
        let mut converter = PcmConverter::new(sink(&device), rate).unwrap();
        for _ in 0..rate / 100 {
            converter.mono(0.5);
        }
        converter.flush();
        let recording = capture.finish().unwrap();
        assert_eq!(recording.pcm.len(), 240, "rate={rate}");
        assert!(recording.pcm.iter().all(|sample| *sample == 16384));
        let wav = recording.wav().unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), SAMPLE_RATE);
        assert_eq!(&wav[20..24], &[1, 0, 1, 0]);
        assert_eq!(&wav[32..36], &[2, 0, 16, 0]);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 480);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 480);
    }
}
#[test]
fn voice_task_04_native_enumeration_does_not_fallback_for_missing_id() {
    let devices = input_devices().unwrap();
    println!("native input devices: {}", devices.len());
    for device in &devices {
        assert!(!device.id.is_empty());
        assert!(!device.name.is_empty());
    }
    let lease = Arc::new(AtomicBool::new(false));
    let result = Capture::start_backend(
        super::capture_native::NativeCapture,
        Some("missing-test-device-id".into()),
        Duration::from_secs(1),
        lease.clone(),
    );
    assert_eq!(result.err(), Some(CaptureError::DeviceRemoved));
    assert!(!lease.load(Ordering::Acquire));
}

#[test]
fn voice_task_04_typed_stereo_conversion_preserves_callback_boundaries() {
    let device = device();
    let capture = start(&device, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
    let mut converter = PcmConverter::new(sink(&device), 44_100).unwrap();
    // Three callbacks of 147 stereo frames produce 240 mono samples at 24 kHz.
    for _ in 0..3 {
        super::capture_native::ingest(&mut converter, &[16384_i16, 0].repeat(147), 2);
    }
    assert_eq!(capture.finish().unwrap().pcm, vec![8192; 240]);
    for sample in [f32::NAN, f32::INFINITY] {
        let device = self::device();
        let capture = start(&device, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
        let mut converter = PcmConverter::new(sink(&device), SAMPLE_RATE).unwrap();
        super::capture_native::ingest(&mut converter, &[sample; 480], 1);
        assert_eq!(capture.finish().unwrap_err(), CaptureError::Silent);
    }
    let device = self::device();
    let capture = start(&device, Arc::new(AtomicBool::new(false)), Duration::from_secs(1));
    let mut converter = PcmConverter::new(sink(&device), SAMPLE_RATE).unwrap();
    super::capture_native::ingest(&mut converter, &[49152_u16; 480], 1);
    super::capture_native::ingest(&mut converter, &[1073741824_i32; 480], 1);
    assert_eq!(capture.finish().unwrap().pcm, vec![16384; 960]);
}
#[test]
fn voice_task_04_device_start_errors_release_lease_and_remain_explicit() {
    struct Unavailable;
    impl CaptureBackend for Unavailable {
        type Stream = ();
        fn open(self, _: Option<String>, _: CaptureSink) -> Result<(), CaptureError> {
            Err(CaptureError::Permission)
        }
    }
    let lease = Arc::new(AtomicBool::new(false));
    assert_eq!(
        Capture::start_backend(Unavailable, None, Duration::from_secs(1), lease.clone()).err(),
        Some(CaptureError::Permission)
    );
    assert!(!lease.load(Ordering::Acquire));
    let device = device();
    let mut capture = start(&device, lease, Duration::from_secs(1));
    sink(&device).error(CaptureError::Device);
    assert_eq!(await_result(&mut capture).unwrap_err(), CaptureError::Device);
    assert!(device.released.load(Ordering::Acquire));
}
