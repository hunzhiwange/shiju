//! Bounded extraction from a local video attachment using the configured FFmpeg executable.

use std::{path::Path, process::Stdio, time::Duration};

use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;

use super::{Code, MediaFile, blocking};

const FRAME_LIMIT: usize = 6;
const FRAME_BYTES: usize = 4 * 1024 * 1024;
const AUDIO_BYTES: usize = 2 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(25);

pub(super) fn video_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.get(4..8) == Some(b"ftyp") {
        Some("video/mp4")
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        Some("video/webm")
    } else {
        None
    }
}

pub(super) async fn extract(
    executable: &Path,
    video: MediaFile,
    cancel: &CancellationToken,
) -> Result<(Vec<MediaFile>, Option<MediaFile>), Code> {
    let format = match (video.mime.as_str(), video_mime(&video.bytes)) {
        ("video/mp4" | "video/quicktime", Some("video/mp4")) => "mov",
        ("video/webm", Some("video/webm")) => "matroska",
        _ => return Err(Code::UnsupportedMedia),
    };
    let source = blocking(move || {
        use std::io::Write as _;
        let mut file = tempfile::NamedTempFile::new().map_err(|_| Code::Unavailable)?;
        file.write_all(&video.bytes).map_err(|_| Code::Unavailable)?;
        Ok(file)
    })
    .await?;
    let work = async {
        let frames = run(
            executable,
            source.path(),
            format,
            &[
                "-an",
                "-vf",
                "fps=1/5:start_time=0,scale=640:640:force_original_aspect_ratio=decrease",
                "-frames:v",
                "6",
                "-c:v",
                "mjpeg",
                "-f",
                "image2pipe",
            ],
            FRAME_BYTES,
        )
        .await?;
        let audio = match run(
            executable,
            source.path(),
            format,
            &["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav"],
            AUDIO_BYTES,
        )
        .await
        {
            Ok(bytes)
                if bytes.len() >= 44 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE" =>
            {
                // A WAV written to a pipe contains sentinel lengths.
                Some(MediaFile { mime: "audio/wav".into(), bytes: finalize_wav(bytes)? })
            }
            Err(Code::UnsupportedMedia) => None,
            Err(error) => return Err(error),
            Ok(_) => return Err(Code::UnsupportedMedia),
        };
        Ok((decode_frames(&frames)?, audio))
    };
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(Code::Conflict),
        result = tokio::time::timeout(DEADLINE, work) => result.unwrap_or(Err(Code::Unavailable)),
    }
}

async fn run(
    executable: &Path,
    input: &Path,
    format: &str,
    options: &[&str],
    limit: usize,
) -> Result<Vec<u8>, Code> {
    let mut command = Command::new(executable);
    command
        .env_clear()
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-max_alloc",
            "67108864",
            "-protocol_whitelist",
            "file,pipe",
            "-f",
            format,
        ]);
    if format == "mov" {
        command.args(["-enable_drefs", "0"]);
    }
    let mut child = command
        .arg("-i")
        .arg(input)
        .args(["-t", "30"])
        .args(options)
        .arg("pipe:1")
        .spawn()
        .map_err(|_| Code::Unavailable)?;
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .ok_or(Code::Unavailable)?
        .take((limit + 1) as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|_| Code::Unavailable)?;
    if output.len() > limit {
        return Err(Code::PayloadTooLarge);
    }
    if !child.wait().await.map_err(|_| Code::Unavailable)?.success() || output.is_empty() {
        return Err(Code::UnsupportedMedia);
    }
    Ok(output)
}

fn decode_frames(mut bytes: &[u8]) -> Result<Vec<MediaFile>, Code> {
    let mut frames = Vec::new();
    while !bytes.is_empty() {
        if frames.len() == FRAME_LIMIT || !bytes.starts_with(&[0xff, 0xd8]) {
            return Err(Code::UnsupportedMedia);
        }
        let length =
            bytes.windows(2).position(|pair| pair == [0xff, 0xd9]).ok_or(Code::UnsupportedMedia)?
                + 2;
        let (frame, rest) = bytes.split_at(length);
        frames.push(MediaFile { mime: "image/jpeg".into(), bytes: frame.to_vec() });
        bytes = rest;
    }
    if frames.is_empty() {
        return Err(Code::UnsupportedMedia);
    }
    Ok(frames)
}

fn finalize_wav(mut bytes: Vec<u8>) -> Result<Vec<u8>, Code> {
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let size_position = offset + 4;
        let data_position = offset + 8;
        if &bytes[offset..size_position] == b"data" {
            let data_size = (bytes.len() - data_position) as u32;
            if data_size == 0 {
                return Err(Code::UnsupportedMedia);
            }
            let riff_size = (bytes.len() - 8) as u32;
            bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
            bytes[size_position..data_position].copy_from_slice(&data_size.to_le_bytes());
            return Ok(bytes);
        }
        let size =
            u32::from_le_bytes(bytes[size_position..data_position].try_into().unwrap()) as usize;
        offset = data_position
            .checked_add(size)
            .and_then(|value| value.checked_add(size % 2))
            .ok_or(Code::UnsupportedMedia)?;
    }
    Err(Code::UnsupportedMedia)
}
