//! Bounded decoding of buffered HTTP entities for application routing.

use async_compression::{
    tokio::bufread::{BrotliDecoder, DeflateDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder},
    zstd::DParameter,
};
use axum::{
    body::Bytes,
    http::{HeaderMap, header},
};
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DecodeError {
    UnsupportedCoding,
    Corrupt,
    TooLarge,
}

#[derive(Clone, Copy)]
enum Coding {
    Gzip,
    Deflate,
    Brotli,
    Zstd,
}

pub(super) struct ContentCodings(Vec<Coding>);

impl ContentCodings {
    /// Combine repeated fields in wire order before interpreting the coding list.
    pub(super) fn from_headers(headers: &HeaderMap) -> Result<Self, DecodeError> {
        let mut codings = Vec::new();
        for field in headers.get_all(header::CONTENT_ENCODING) {
            let field = field.to_str().map_err(|_| DecodeError::UnsupportedCoding)?;
            for token in field.split(',').map(str::trim) {
                codings.push(match token.to_ascii_lowercase().as_str() {
                    "" | "identity" => continue,
                    "gzip" | "x-gzip" => Coding::Gzip,
                    "deflate" => Coding::Deflate,
                    "br" => Coding::Brotli,
                    "zstd" | "zst" => Coding::Zstd,
                    _ => return Err(DecodeError::UnsupportedCoding),
                });
            }
        }
        Ok(Self(codings))
    }

    /// Decode in reverse application order. Every intermediate output and the
    /// final entity must fit within `limit`. The caller limits wire collection.
    pub(super) async fn decode(self, mut body: Bytes, limit: usize) -> Result<Bytes, DecodeError> {
        for coding in self.0.into_iter().rev() {
            tokio::task::consume_budget().await;
            body = coding.decode(&body, limit).await?.into();
        }
        if body.len() > limit {
            return Err(DecodeError::TooLarge);
        }
        Ok(body)
    }
}

impl Coding {
    async fn decode(self, input: &[u8], limit: usize) -> Result<Vec<u8>, DecodeError> {
        let mut remaining = input;
        let output = match self {
            Self::Gzip => {
                let mut decoder = GzipDecoder::new(&mut remaining);
                decoder.multiple_members(true);
                read_bounded(decoder, limit).await?
            }
            Self::Deflate => {
                match read_bounded(ZlibDecoder::new(&mut remaining), limit).await {
                    Ok(output) if remaining.is_empty() => return Ok(output),
                    Err(DecodeError::TooLarge) => return Err(DecodeError::TooLarge),
                    // Some clients omit the zlib wrapper.
                    _ => {
                        remaining = input;
                        read_bounded(DeflateDecoder::new(&mut remaining), limit).await?
                    }
                }
            }
            Self::Brotli => read_bounded(BrotliDecoder::new(&mut remaining), limit).await?,
            Self::Zstd => {
                // Bound the advertised decoder window independently of the output.
                let params = [DParameter::window_log_max(23)];
                let mut decoder = ZstdDecoder::with_params(&mut remaining, &params);
                decoder.multiple_members(true);
                read_bounded(decoder, limit).await?
            }
        };
        if !remaining.is_empty() {
            return Err(DecodeError::Corrupt);
        }
        Ok(output)
    }
}

async fn read_bounded(
    decoder: impl AsyncRead + Unpin,
    limit: usize,
) -> Result<Vec<u8>, DecodeError> {
    let mut output = Vec::new();
    decoder
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|_| DecodeError::Corrupt)?;
    if output.len() > limit {
        return Err(DecodeError::TooLarge);
    }
    Ok(output)
}
