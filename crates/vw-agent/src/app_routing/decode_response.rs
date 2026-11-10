//! Bounded collection and decoding of non-streaming upstream responses.

use super::{
    body_limits::{self, BodyError},
    decode_request::{ContentCodings, DecodeError},
};
use axum::body::{Body, Bytes};

pub(super) async fn decode(
    body: Body,
    codings: ContentCodings,
    limit: usize,
) -> Result<Bytes, DecodeError> {
    let wire = body_limits::collect(body, limit).await.map_err(|error| match error {
        BodyError::Malformed => DecodeError::Corrupt,
        BodyError::TooLarge => DecodeError::TooLarge,
    })?;
    codings.decode(wire, limit).await
}
