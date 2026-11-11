//! Bounded 24 kHz mono PCM16 playback. The callback never waits for the producer.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};

const RATE: u32 = 24_000;
const CAPACITY: usize = RATE as usize * 2;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
    Stopped,
    Format,
    Overflow,
    Device,
}
type Result<T> = std::result::Result<T, PlaybackError>;
struct Buffer {
    samples: VecDeque<i16>,
    active: bool,
    error: Option<PlaybackError>,
}
#[derive(Clone)]
pub(super) struct Output {
    buffer: Arc<Mutex<Buffer>>,
    rate: u32,
    channels: usize,
    phase: u32,
    current: Option<i16>,
}
impl Output {
    fn new(rate: u32, channels: usize) -> Result<Self> {
        if !(8_000..=192_000).contains(&rate) || !(1..=32).contains(&channels) {
            return Err(PlaybackError::Format);
        }
        Ok(Self {
            buffer: Arc::new(Mutex::new(Buffer {
                samples: VecDeque::with_capacity(CAPACITY),
                active: true,
                error: None,
            })),
            rate,
            channels,
            phase: 0,
            current: None,
        })
    }
    pub(super) fn fail(&self, error: PlaybackError) {
        let mut b = self.buffer.lock();
        b.active = false;
        b.samples.clear();
        b.error = Some(error);
    }
    pub(super) fn render<T>(&mut self, data: &mut [T])
    where
        T: cpal::Sample + cpal::FromSample<f32>,
    {
        data.fill(T::from_sample(0.0));
        let Some(mut buffer) = self.buffer.try_lock() else {
            return;
        };
        if !buffer.active {
            self.current = None;
            self.phase = 0;
            return;
        }
        // Hold the short buffer guard through rendering: stop waits for any in-flight
        // callback before invalidating this generation. No old callback can refill it.
        for frame in data.chunks_exact_mut(self.channels) {
            if self.current.is_none() {
                self.current = buffer.samples.pop_front();
            }
            let sample = self.current.unwrap_or(0) as f32 / 32768.0;
            frame.fill(T::from_sample(sample));
            self.phase += RATE;
            while self.phase >= self.rate {
                self.phase -= self.rate;
                if self.phase >= self.rate {
                    buffer.samples.pop_front();
                }
                self.current = None;
            }
        }
    }
}
/// Each instance owns a generation; stopping permanently invalidates its callback and queue.
pub struct Playback<S = cpal::Stream> {
    output: Output,
    stream: Option<S>,
}
impl Playback {
    pub fn start() -> Result<Self> {
        let device = cpal::default_host().default_output_device().ok_or(PlaybackError::Device)?;
        let config = device.default_output_config().map_err(|_| PlaybackError::Device)?;
        let output = Output::new(config.sample_rate(), config.channels() as usize)?;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, config.config(), output.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, config.config(), output.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, config.config(), output.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, config.config(), output.clone()),
            _ => Err(PlaybackError::Format),
        }?;
        stream.play().map_err(|_| PlaybackError::Device)?;
        Ok(Self { output, stream: Some(stream) })
    }
}
impl<S> Playback<S> {
    /// Accept only contract PCM s16le, 24 kHz, mono. A packet is at most 200 ms.
    pub fn push_pcm(&mut self, bytes: &[u8]) -> Result<()> {
        let mut b = self.output.buffer.lock();
        if !b.active {
            let error = b.error.unwrap_or(PlaybackError::Stopped);
            drop(b);
            self.stream.take();
            return Err(error);
        }
        let error = if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
            Some(PlaybackError::Format)
        } else if bytes.len() > 9600 || b.samples.len() + bytes.len() / 2 > CAPACITY {
            Some(PlaybackError::Overflow)
        } else {
            None
        };
        if let Some(error) = error {
            b.active = false;
            b.samples.clear();
            b.error = Some(error);
            drop(b);
            self.stream.take();
            return Err(error);
        }
        b.samples.extend(bytes.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]])));
        Ok(())
    }
    /// Poll on the owning thread to release a failed native stream.
    pub fn error(&mut self) -> Option<PlaybackError> {
        let error = self.output.buffer.lock().error;
        if error.is_some() {
            self.stream.take();
        }
        error
    }
    pub fn stop(&mut self) {
        let mut buffer = self.output.buffer.lock();
        buffer.active = false;
        buffer.samples.clear();
        drop(buffer);
        self.stream.take();
    }
}
impl<S> Drop for Playback<S> {
    fn drop(&mut self) {
        self.stop();
    }
}
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut output: Output,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let errors = output.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| output.render(data),
            move |_| errors.fail(PlaybackError::Device),
            None,
        )
        .map_err(|_| PlaybackError::Device)
}
#[cfg(test)]
pub(super) fn fixture(rate: u32, channels: usize) -> (Playback<()>, Output) {
    fixture_stream(rate, channels, ())
}
#[cfg(test)]
pub(super) fn fixture_stream<S>(rate: u32, channels: usize, stream: S) -> (Playback<S>, Output) {
    let output = Output::new(rate, channels).unwrap();
    (Playback { output: output.clone(), stream: Some(stream) }, output)
}
impl std::fmt::Display for PlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stopped => "语音播放已停止",
            Self::Format => "语音播放格式不受支持",
            Self::Overflow => "语音播放缓冲区已满",
            Self::Device => "扬声器不可用或已断开",
        })
    }
}
impl std::error::Error for PlaybackError {}
