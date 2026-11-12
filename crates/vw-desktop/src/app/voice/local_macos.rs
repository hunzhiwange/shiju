//! OS-only boundary. No network fallback, model download, or implicit authorization.
use super::super::permissions::{Permission, PermissionBackend, SystemPermissions};
use super::*;
use block2::RcBlock;
use objc2::rc::autoreleasepool;
use objc2::{
    msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_foundation::NSString;
use std::{
    ffi::CStr,
    io::Write,
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

#[link(name = "AVFAudio", kind = "framework")]
unsafe extern "C" {}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: *const std::ffi::c_void;
    fn CFRunLoopRunInMode(
        mode: *const std::ffi::c_void,
        seconds: f64,
        return_after_source: bool,
    ) -> i32;
}

fn class(name: &CStr) -> Result<&'static AnyClass> {
    AnyClass::get(name).ok_or(LocalSpeechError::Unavailable)
}
fn recognizer(locale: Option<&str>) -> Result<Retained<AnyObject>> {
    // SAFETY: All selectors/signatures below are SDK-declared Foundation/Speech
    // methods. Owned returns retain objects; NSString arguments live through calls.
    unsafe {
        let class = super::super::permissions_macos::speech_class();
        let object: Option<Retained<AnyObject>> = if let Some(locale) = locale {
            let locale: Retained<AnyObject> = msg_send![self::class(c"NSLocale")?, localeWithLocaleIdentifier: &*NSString::from_str(locale)];
            let allocated: Allocated<AnyObject> = msg_send![class, alloc];
            msg_send![allocated, initWithLocale: &*locale]
        } else {
            msg_send![class, new]
        };
        object.ok_or(LocalSpeechError::UnsupportedLanguage)
    }
}
fn recognition_state(recognizer: &AnyObject) -> Capabilities {
    // SAFETY: recognizer was created by SFSpeechRecognizer above; borrowed result
    // objects remain alive in this autorelease pool while copied to Rust strings.
    unsafe {
        let locale: *mut AnyObject = msg_send![recognizer, locale];
        let identifier: *mut NSString = msg_send![locale, localeIdentifier];
        let offline: Bool = msg_send![recognizer, supportsOnDeviceRecognition];
        let available: Bool = msg_send![recognizer, isAvailable];
        Capabilities {
            language: (&*identifier).to_string(),
            supports_on_device: offline.as_bool(),
            available: available.as_bool(),
            permission: SystemPermissions.query(Permission::SpeechRecognition),
            voices: vec![],
        }
    }
}
fn system_voices() -> Result<Vec<SystemVoice>> {
    // SAFETY: SDK NSArray of AVSpeechSynthesisVoice; objects borrowed from the
    // retained array. Only installed Apple voices are eligible, excluding plugins.
    unsafe {
        let voices: Retained<AnyObject> =
            msg_send![class(c"AVSpeechSynthesisVoice")?, speechVoices];
        let count: usize = msg_send![&*voices, count];
        if count > 4096 {
            return Err(LocalSpeechError::Limit);
        }
        let mut result = Vec::with_capacity(count);
        for index in 0..count {
            let voice: *mut AnyObject = msg_send![&*voices, objectAtIndex: index];
            let id: *mut NSString = msg_send![voice, identifier];
            let id = (&*id).to_string();
            if !(id.starts_with("com.apple.voice.")
                || id.starts_with("com.apple.speech.synthesis.voice.")
                || id.starts_with("com.apple.ttsbundle."))
            {
                continue;
            }
            let installed: *mut AnyObject = msg_send![class(c"AVSpeechSynthesisVoice")?, voiceWithIdentifier: &*NSString::from_str(&id)];
            if installed.is_null() {
                continue;
            }
            let name: *mut NSString = msg_send![voice, name];
            let language: *mut NSString = msg_send![voice, language];
            result.push(SystemVoice {
                id,
                name: (&*name).to_string(),
                language: (&*language).to_string(),
            });
        }
        Ok(result)
    }
}
pub(super) fn voices() -> Result<Vec<SystemVoice>> {
    autoreleasepool(|_| system_voices())
}
pub(super) fn capabilities(locale: Option<&str>) -> Result<Capabilities> {
    autoreleasepool(|_| {
        let recognizer = recognizer(locale)?;
        let mut state = recognition_state(&recognizer);
        state.voices = system_voices()?;
        Ok(state)
    })
}

enum JobKind {
    Recognition,
    Synthesis,
}
struct NativeJob {
    object: Retained<AnyObject>,
    kind: JobKind,
    active: Arc<AtomicBool>,
}
impl Drop for NativeJob {
    fn drop(&mut self) {
        // Gate callbacks first, then ask the system to stop and release ownership.
        self.active.store(false, Ordering::Release);
        // SAFETY: Each object is paired with its SDK class at creation below.
        unsafe {
            match self.kind {
                JobKind::Recognition => {
                    let _: () = msg_send![&*self.object, cancel];
                }
                JobKind::Synthesis => {
                    let _: Bool = msg_send![&*self.object, stopSpeakingAtBoundary: 0isize];
                }
            }
        }
    }
}
fn wait<T>(receiver: mpsc::Receiver<Result<T>>, cancel: &Cancellation, seconds: u64) -> Result<T> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        cancel.check()?;
        match receiver.try_recv() {
            Ok(result) => return cancel.complete(result),
            Err(mpsc::TryRecvError::Disconnected) => return Err(LocalSpeechError::Unavailable),
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if Instant::now() >= deadline {
            return Err(LocalSpeechError::Timeout);
        }
        // Framework callbacks may need a run loop. Each worker pumps at most
        // 20 ms between cancellation checks, never the Iced UI thread.
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.02, true);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn wav_file(pcm: &[i16]) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|_| LocalSpeechError::Storage)?;
    let bytes = (pcm.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(bytes + 36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(b"\x02\0\x10\0data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    for sample in pcm {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    file.write_all(&wav).map_err(|_| LocalSpeechError::Storage)?;
    Ok(file)
}
fn request_for_path(path: &str) -> Result<Retained<AnyObject>> {
    // SAFETY: NSURL/SFSpeechURLRecognitionRequest SDK constructors. Explicitly
    // force local mode before task creation; partial results never escape.
    unsafe {
        let url: Retained<AnyObject> =
            msg_send![class(c"NSURL")?, fileURLWithPath: &*NSString::from_str(path)];
        let allocated: Allocated<AnyObject> =
            msg_send![class(c"SFSpeechURLRecognitionRequest")?, alloc];
        let request: Retained<AnyObject> = msg_send![allocated, initWithURL: &*url];
        let _: () = msg_send![&*request, setRequiresOnDeviceRecognition: Bool::YES];
        let _: () = msg_send![&*request, setShouldReportPartialResults: Bool::NO];
        let offline: Bool = msg_send![&*request, requiresOnDeviceRecognition];
        if !offline.as_bool() {
            return Err(LocalSpeechError::OfflineUnavailable);
        }
        Ok(request)
    }
}
pub(super) fn transcribe(
    pcm: &[i16],
    locale: Option<&str>,
    cancel: &Cancellation,
) -> Result<Transcript> {
    autoreleasepool(|_| {
        let recognizer = recognizer(locale)?;
        let state = recognition_state(&recognizer);
        recognition_gate(&state)?;
        if !super::super::permissions_macos::has_usage_description(
            "NSSpeechRecognitionUsageDescription",
        ) {
            return Err(LocalSpeechError::AppMetadata);
        }
        cancel.check()?;
        let file = wav_file(pcm)?; // private 0600 temp file, deleted on every exit
        let path = file.path().to_str().ok_or(LocalSpeechError::InvalidAudio)?;
        let request = request_for_path(path)?;
        let active = Arc::new(AtomicBool::new(true));
        let live = active.clone();
        let token = cancel.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        // SAFETY: Speech retains the result handler. Its pointer arguments are
        // valid only within the callback; copy final text, never retain raw data.
        unsafe {
            let callback = RcBlock::new(move |result: *mut AnyObject, error: *mut AnyObject| {
                if !live.load(Ordering::Acquire) || token.is_cancelled() {
                    return;
                }
                if !error.is_null() {
                    let code: isize = msg_send![error, code];
                    let _ = sender.try_send(Err(LocalSpeechError::System(code as i64)));
                    return;
                }
                if result.is_null() {
                    return;
                }
                let final_result: Bool = msg_send![result, isFinal];
                if !final_result.as_bool() {
                    return;
                }
                let best: *mut AnyObject = msg_send![result, bestTranscription];
                let text: *mut NSString = msg_send![best, formattedString];
                let length: usize = msg_send![text, length];
                let output = if length > 16_000 {
                    Err(LocalSpeechError::Limit)
                } else {
                    let text = (&*text).to_string();
                    if text.trim().is_empty() {
                        Err(LocalSpeechError::EmptyResult)
                    } else {
                        Ok(text)
                    }
                };
                let _ = sender.try_send(output);
            });
            let queue: Retained<AnyObject> = msg_send![class(c"NSOperationQueue")?, new];
            let _: () = msg_send![&*recognizer, setQueue: &*queue];
            let task: Option<Retained<AnyObject>> = msg_send![&*recognizer, recognitionTaskWithRequest: &*request, resultHandler: &*callback];
            let _job = NativeJob {
                object: task.ok_or(LocalSpeechError::Unavailable)?,
                kind: JobKind::Recognition,
                active,
            };
            let text = wait(receiver, cancel, 75)?;
            cancel.complete(Ok(Transcript { text, language: state.language }))
        }
    })
}

pub(super) fn synthesize(
    text: &str,
    voice_id: Option<&str>,
    cancel: &Cancellation,
) -> Result<SpeechAudio> {
    autoreleasepool(|_| {
        let voices = system_voices()?;
        // SAFETY: Use only a validated, installed Apple voice; no default third
        // party provider or silent fallback to a different selected voice.
        unsafe {
            let default: *mut NSString =
                msg_send![class(c"AVSpeechSynthesisVoice")?, currentLanguageCode];
            let language = (&*default).to_string();
            let selected = if let Some(id) = voice_id {
                voices.iter().find(|v| v.id == id)
            } else {
                voices.iter().find(|v| v.language == language)
            }
            .ok_or(LocalSpeechError::VoiceUnavailable)?;
            let voice: Option<Retained<AnyObject>> = msg_send![class(c"AVSpeechSynthesisVoice")?, voiceWithIdentifier: &*NSString::from_str(&selected.id)];
            let voice = voice.ok_or(LocalSpeechError::VoiceUnavailable)?;
            let utterance: Retained<AnyObject> = msg_send![class(c"AVSpeechUtterance")?, speechUtteranceWithString: &*NSString::from_str(text)];
            let _: () = msg_send![&*utterance, setVoice: &*voice];
            let synthesizer: Retained<AnyObject> = msg_send![class(c"AVSpeechSynthesizer")?, new];
            let active = Arc::new(AtomicBool::new(true));
            let live = active.clone();
            let token = cancel.clone();
            let audio = Arc::new(Mutex::new(PcmOutput::default()));
            let collected = audio.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let callback = RcBlock::new(move |buffer: *mut AnyObject| {
                if !live.load(Ordering::Acquire) || token.is_cancelled() || buffer.is_null() {
                    return;
                }
                let Some(pcm_class) = AnyClass::get(c"AVAudioPCMBuffer") else {
                    let _ = sender.try_send(Err(LocalSpeechError::Unavailable));
                    return;
                };
                let is_pcm: Bool = msg_send![buffer, isKindOfClass: pcm_class];
                if !is_pcm.as_bool() {
                    let _ = sender.try_send(Err(LocalSpeechError::UnsupportedSynthesisFormat));
                    return;
                }
                let frames: u32 = msg_send![buffer, frameLength];
                if frames == 0 {
                    let _ = sender.try_send(Ok(()));
                    return;
                }
                let Ok(mut out) = collected.lock() else {
                    let _ = sender.try_send(Err(LocalSpeechError::Unavailable));
                    return;
                };
                let format: *mut AnyObject = msg_send![buffer, format];
                let rate: f64 = msg_send![format, sampleRate];
                let channels: u32 = msg_send![format, channelCount];
                let interleaved: Bool = msg_send![format, isInterleaved];
                let common_format: isize = msg_send![format, commonFormat];
                let data: *const *const f32 = msg_send![buffer, floatChannelData];
                if frames > 192_000
                    || channels == 0
                    || channels > 16
                    || common_format != 1
                    || interleaved.as_bool()
                    || data.is_null()
                    || !rate.is_finite()
                    || rate.fract() != 0.0
                {
                    let _ = sender.try_send(Err(LocalSpeechError::UnsupportedSynthesisFormat));
                    return;
                }
                for channel in 0..channels as usize {
                    if (*data.add(channel)).is_null() {
                        let _ = sender.try_send(Err(LocalSpeechError::UnsupportedSynthesisFormat));
                        return;
                    }
                }
                for frame in 0..frames as usize {
                    if frame % 256 == 0 && (!live.load(Ordering::Acquire) || token.is_cancelled()) {
                        return;
                    }
                    let mono = (0..channels as usize)
                        .map(|channel| *(*data.add(channel)).add(frame) as f64)
                        .sum::<f64>()
                        / channels as f64;
                    if let Err(error) = out.push(mono, rate as u32) {
                        let _ = sender.try_send(Err(error));
                        break;
                    }
                }
            });
            let job = NativeJob { object: synthesizer, kind: JobKind::Synthesis, active };
            cancel.check()?;
            let _: () =
                msg_send![&*job.object, writeUtterance: &*utterance, toBufferCallback: &*callback];
            wait(receiver, cancel, MAX_SYNTHESIS_SECONDS as u64 + 5)?;
            drop(job); // suppress late callback writes before extracting output
            let pcm =
                std::mem::take(&mut audio.lock().map_err(|_| LocalSpeechError::Unavailable)?.pcm);
            if pcm.is_empty() {
                return Err(LocalSpeechError::EmptyResult);
            }
            cancel.complete(Ok(SpeechAudio { pcm }))
        }
    })
}

#[cfg(test)]
pub(super) fn offline_request_probe() -> Result<bool> {
    autoreleasepool(|_| {
        let file = wav_file(&[0; 24])?;
        let request =
            request_for_path(file.path().to_str().ok_or(LocalSpeechError::InvalidAudio)?)?;
        // SAFETY: inspect the same production request constructor, no task/start.
        unsafe {
            let offline: Bool = msg_send![&*request, requiresOnDeviceRecognition];
            let partial: Bool = msg_send![&*request, shouldReportPartialResults];
            Ok(offline.as_bool() && !partial.as_bool())
        }
    })
}
