//! Parse only bounded recording bytes and typed metadata, never paths or URLs.
use axum::extract::{FromRequest, Multipart, Request};
use vw_api_types::voice::{
    MAX_METADATA_BYTES, MAX_WAV_BYTES, TranscriptionRequest, VoiceErrorCode as Code,
};
pub(super) const MAX_BODY: usize = 24_100_000;
pub(super) async fn read_upload(request: Request) -> Result<(TranscriptionRequest, Vec<u8>), Code> {
    // Count the actual stream length, including multipart overhead, even
    // when Content-Length is absent or false. The outer deadline also bounds stalls.
    let (parts, body) = request.into_parts();
    use futures_util::StreamExt;
    let mut stream = body.into_data_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| Code::InvalidRequest)?;
        if bytes.len() + chunk.len() > MAX_BODY {
            return Err(Code::PayloadTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    let request = Request::from_parts(parts, axum::body::Body::from(bytes));
    let mut multipart =
        Multipart::from_request(request, &()).await.map_err(|_| Code::InvalidRequest)?;
    let mut metadata = None;
    let mut audio = None;
    while let Some(field) = multipart.next_field().await.map_err(|_| Code::InvalidRequest)? {
        match field.name() {
            Some("metadata") if metadata.is_none() => {
                let bytes = field.bytes().await.map_err(|_| Code::InvalidRequest)?;
                if bytes.len() > MAX_METADATA_BYTES {
                    return Err(Code::PayloadTooLarge);
                }
                metadata = Some(serde_json::from_slice(&bytes).map_err(|_| Code::InvalidRequest)?);
            }
            Some("file") if audio.is_none() => {
                if field.content_type() != Some("audio/wav") {
                    return Err(Code::UnsupportedMedia);
                }
                let bytes = field.bytes().await.map_err(|_| Code::InvalidRequest)?;
                if bytes.len() > MAX_WAV_BYTES {
                    return Err(Code::PayloadTooLarge);
                }
                audio = Some(bytes.to_vec());
            }
            _ => return Err(Code::InvalidRequest),
        }
    }
    Ok((metadata.ok_or(Code::InvalidRequest)?, audio.ok_or(Code::InvalidRequest)?))
}

#[cfg(test)]
#[path = "voice_upload_tests.rs"]
mod tests;
