//! Bounded decoding of compressed, non-streaming upstream responses.

use super::decode_request::{ContentCodings, DecodeError};
use axum::body::Body;
use futures_util::StreamExt;

pub(super) async fn decode(
    body: Body,
    codings: ContentCodings,
    limit: usize,
) -> Result<Body, DecodeError> {
    if codings.is_empty() {
        return Ok(body);
    }

    let mut wire = Vec::new();
    let mut chunks = body.into_data_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|_| DecodeError::Corrupt)?;
        if chunk.len() > limit - wire.len() {
            return Err(DecodeError::TooLarge);
        }
        wire.extend_from_slice(&chunk);
    }
    codings.decode(wire.into(), limit).await.map(Body::from)
}
