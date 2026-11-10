//! Admission and byte budgets for buffered application routing.

use axum::{
    body::{Body, Bytes},
    http::{HeaderMap, header},
};
use futures_util::StreamExt;
use std::{io, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(super) const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_BUFFERED_REQUESTS: usize = 8;

#[derive(Clone)]
pub(super) struct BodyLimits(Arc<Semaphore>);

impl Default for BodyLimits {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_BUFFERED_REQUESTS)))
    }
}

impl BodyLimits {
    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn available(&self) -> usize {
        self.0.available_permits()
    }

    pub(super) fn try_reserve(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.0).try_acquire_owned().ok()
    }
}

pub(super) fn buffered_response(bytes: Bytes, permit: OwnedSemaphorePermit) -> Body {
    // Keep the reservation until the last reference to the response bytes is
    // dropped, including while the HTTP writer waits for a slow client.
    Body::from(Bytes::from_owner(BufferedBody { bytes, _permit: permit }))
}

struct BufferedBody {
    bytes: Bytes,
    _permit: OwnedSemaphorePermit,
}

impl AsRef<[u8]> for BufferedBody {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug)]
pub(super) enum BodyError {
    Malformed,
    TooLarge,
}

pub(super) fn check_declared_length(headers: &HeaderMap, limit: usize) -> Result<(), BodyError> {
    if headers.contains_key(header::CONTENT_LENGTH)
        && headers.contains_key(header::TRANSFER_ENCODING)
    {
        return Err(BodyError::Malformed);
    }
    let mut length = None;
    for field in headers.get_all(header::CONTENT_LENGTH) {
        for value in field.to_str().map_err(|_| BodyError::Malformed)?.split(',') {
            let value = value.trim_ascii();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(BodyError::Malformed);
            }
            let number = value.parse::<u64>().map_err(|_| BodyError::Malformed)?;
            if length.is_some_and(|previous| previous != number) {
                return Err(BodyError::Malformed);
            }
            length = Some(number);
        }
    }
    if length.is_some_and(|length| length > limit as u64) {
        return Err(BodyError::TooLarge);
    }
    Ok(())
}

pub(super) async fn collect(body: Body, limit: usize) -> Result<Bytes, BodyError> {
    let mut output = Buffer::new(limit);
    let mut chunks = body.into_data_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|_| BodyError::Malformed)?;
        output.extend(&chunk)?;
    }
    Ok(output.bytes.into())
}

pub(super) fn encode_json(value: &serde_json::Value, limit: usize) -> Result<Vec<u8>, BodyError> {
    let mut output = Buffer::new(limit);
    serde_json::to_writer(&mut output, value).map_err(|error| {
        if error.io_error_kind() == Some(io::ErrorKind::FileTooLarge) {
            BodyError::TooLarge
        } else {
            BodyError::Malformed
        }
    })?;
    Ok(output.bytes)
}

struct Buffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Buffer {
    fn new(limit: usize) -> Self {
        Self { bytes: Vec::new(), limit }
    }

    fn extend(&mut self, bytes: &[u8]) -> Result<(), BodyError> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(BodyError::TooLarge);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.extend(bytes).map_err(|_| io::ErrorKind::FileTooLarge)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
