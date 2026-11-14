//! CPAL boundary. Stream remains on its owner thread; callbacks never do IO.
use super::capture::{CaptureBackend, CaptureError, CaptureSink, InputDevice, PcmConverter};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::Duration;
type Result<T> = std::result::Result<T, CaptureError>;

pub(super) fn input_devices() -> Result<Vec<InputDevice>> {
    let host = cpal::default_host();
    let default_id = host.default_input_device().and_then(|device| device.id().ok());
    host.input_devices()
        .map_err(|_| CaptureError::Device)?
        .map(|device| {
            let id = device.id().map_err(|_| CaptureError::DeviceRemoved)?;
            Ok(InputDevice {
                is_default: default_id.as_ref() == Some(&id),
                id: id.to_string(),
                name: device.description().map_err(|_| CaptureError::Device)?.name().to_owned(),
            })
        })
        .collect()
}

pub(super) struct NativeCapture;
impl CaptureBackend for NativeCapture {
    type Stream = cpal::Stream;
    fn open(self, device_id: Option<String>, sink: CaptureSink) -> Result<Self::Stream> {
        let host = cpal::default_host();
        // Resolve default to a concrete enumerated device so route changes cannot follow it.
        let selected = match device_id {
            Some(id) => id,
            None => host
                .default_input_device()
                .ok_or(CaptureError::NoDevice)?
                .id()
                .map_err(|_| CaptureError::Device)?
                .to_string(),
        };
        let device = host
            .input_devices()
            .map_err(|_| CaptureError::Device)?
            .find(|device| device.id().is_ok_and(|id| id.to_string() == selected))
            .ok_or(CaptureError::DeviceRemoved)?;
        let config = device.default_input_config().map_err(map_error)?;
        let stream_config = config.config();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &stream_config, sink),
            cpal::SampleFormat::I16 => build::<i16>(&device, &stream_config, sink),
            cpal::SampleFormat::U16 => build::<u16>(&device, &stream_config, sink),
            cpal::SampleFormat::I32 => build::<i32>(&device, &stream_config, sink),
            _ => Err(CaptureError::UnsupportedFormat),
        }?;
        stream.play().map_err(map_error)?;
        Ok(stream)
    }
}
fn map_error(error: cpal::Error) -> CaptureError {
    match error.kind() {
        cpal::ErrorKind::PermissionDenied => CaptureError::Permission,
        cpal::ErrorKind::DeviceNotAvailable => CaptureError::DeviceRemoved,
        _ => CaptureError::Device,
    }
}
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sink: CaptureSink,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    if channels == 0 || channels > 32 {
        return Err(CaptureError::UnsupportedFormat);
    }
    let mut converter = PcmConverter::new(sink.clone(), config.sample_rate)?;
    device
        .build_input_stream(
            *config,
            move |data: &[T], _| {
                ingest(&mut converter, data, channels);
            },
            move |error| sink.error(map_error(error)),
            Some(Duration::from_secs(2)),
        )
        .map_err(map_error)
}

/// Same typed conversion used by the hardware callback and physical-boundary tests.
pub(super) fn ingest<T>(converter: &mut PcmConverter, data: &[T], channels: usize)
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    for frame in data.chunks_exact(channels) {
        let mono = frame
            .iter()
            .map(|sample| <f32 as cpal::FromSample<T>>::from_sample_(*sample))
            .sum::<f32>()
            / channels as f32;
        converter.mono(mono);
    }
    converter.flush();
}
