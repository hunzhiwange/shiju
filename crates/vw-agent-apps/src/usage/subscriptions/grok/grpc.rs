//! Compatibility path for the original Grok gRPC-web billing response.

use chrono::{DateTime, SecondsFormat};
use reqwest::{RequestBuilder, StatusCode};
use vw_api_types::{
    TimestampMs,
    apps_usage::{UsageQueryErrorCode, UsageQuotaWindow},
};

use super::{number, transport_error, window};

const MAX_RESPONSE_BYTES: usize = 256 * 1024;

pub(super) struct Error {
    pub code: UsageQueryErrorCode,
    pub status: Option<u8>,
}

impl From<UsageQueryErrorCode> for Error {
    fn from(code: UsageQueryErrorCode) -> Self {
        Self { code, status: None }
    }
}

pub(super) async fn query(
    request: RequestBuilder,
    now: TimestampMs,
) -> Result<UsageQuotaWindow, Error> {
    use UsageQueryErrorCode::*;

    let mut response = request.send().await.map_err(transport_error)?;
    match response.status() {
        status if status.is_success() => {}
        StatusCode::UNAUTHORIZED => return Err(UpstreamUnauthorized.into()),
        StatusCode::REQUEST_TIMEOUT => return Err(Timeout.into()),
        StatusCode::TOO_MANY_REQUESTS => return Err(UpstreamRateLimited.into()),
        status if status.is_server_error() => return Err(UpstreamUnavailable.into()),
        _ => return Err(UpstreamRejected.into()),
    }
    if let Some(status) = response.headers().get("grpc-status") {
        check_status(status.to_str().map_err(|_| InvalidResponse)?)?;
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(InvalidResponse.into());
        }
        body.extend_from_slice(&chunk);
    }
    let now = now.0.div_euclid(1000);
    let scan = scan_response(&body, now)?;
    let reset = scan.preferred_reset.or(scan.other_reset);
    let percent = scan
        .percent
        .map(|(_, percent)| percent)
        .or_else(|| (reset.is_some() && scan.has_period && !scan.has_fixed32).then_some(0.0));
    let utilization = percent.and_then(number).ok_or(InvalidResponse)?;
    let name = match reset.map(|at| (at.saturating_sub(now) as f64 / 86_400.0).round() as i64) {
        Some(4..=12) => "weekly_limit",
        Some(20..=45) => "monthly",
        _ => "credits",
    };
    let resets_at = reset
        .and_then(|at| DateTime::from_timestamp(at, 0))
        .map(|at| at.to_rfc3339_opts(SecondsFormat::Secs, true));
    Ok(window(name, Some(utilization), resets_at))
}

fn check_status(raw: &str) -> Result<(), Error> {
    use UsageQueryErrorCode::*;

    let status = raw.trim().parse::<u8>().map_err(|_| InvalidResponse)?;
    let code = match status {
        0 => return Ok(()),
        4 => Timeout,
        8 => UpstreamRateLimited,
        14 => UpstreamUnavailable,
        16 => UpstreamUnauthorized,
        1..=16 => UpstreamRejected,
        _ => return Err(InvalidResponse.into()),
    };
    Err(Error { code, status: Some(status) })
}

fn scan_response(mut body: &[u8], now: i64) -> Result<Scan, Error> {
    use UsageQueryErrorCode::InvalidResponse;

    let mut scan = Scan::default();
    if !matches!(body.first(), Some(0 | 0x80)) {
        scan.fields(body, [0; 5], 0, now).ok_or(InvalidResponse)?;
        return Ok(scan);
    }
    while !body.is_empty() {
        let [flags, a, b, c, d] = *body.split_off(..5).ok_or(InvalidResponse)? else {
            return Err(InvalidResponse.into());
        };
        let len = u32::from_be_bytes([a, b, c, d]) as usize;
        let payload = body.split_off(..len).ok_or(InvalidResponse)?;
        match flags {
            0 => scan.fields(payload, [0; 5], 0, now).ok_or(InvalidResponse)?,
            0x80 if body.is_empty() => {
                let trailers = std::str::from_utf8(payload).map_err(|_| InvalidResponse)?;
                let mut status = None;
                for (name, value) in trailers.lines().filter_map(|line| line.split_once(':')) {
                    if name.eq_ignore_ascii_case("grpc-status") && status.replace(value).is_some() {
                        return Err(InvalidResponse.into());
                    }
                }
                check_status(status.ok_or(InvalidResponse)?)?;
            }
            _ => return Err(InvalidResponse.into()),
        }
    }
    Ok(scan)
}

#[derive(Default, Clone, Copy)]
struct Scan {
    percent: Option<(usize, f64)>,
    preferred_reset: Option<i64>,
    other_reset: Option<i64>,
    has_period: bool,
    has_fixed32: bool,
}

impl Scan {
    // The legacy endpoint has no published descriptor in the pinned source.
    // Preserve the shallowest usage candidate and known reset path. A nested
    // field may be opaque bytes, so keep its candidates only if it parses.
    fn fields(
        &mut self,
        mut body: &[u8],
        mut path: [u64; 5],
        depth: usize,
        now: i64,
    ) -> Option<()> {
        while !body.is_empty() {
            let tag = varint(&mut body)?;
            let field = tag >> 3;
            if !(1..=0x1fff_ffff).contains(&field) {
                return None;
            }
            path[depth] = field;
            let path_slice = &path[..=depth];
            match tag & 7 {
                0 => {
                    let value = varint(&mut body)?;
                    self.has_period |= path_slice.starts_with(&[1, 6])
                        || (path_slice == [1, 8, 1] && matches!(value, 1 | 2));
                    if (1_700_000_000..=2_100_000_000).contains(&value) && value as i64 > now {
                        let target = if path_slice == [1, 5, 1] {
                            &mut self.preferred_reset
                        } else {
                            &mut self.other_reset
                        };
                        let value = value as i64;
                        *target = Some(target.map_or(value, |previous| previous.min(value)));
                    }
                }
                1 => {
                    body.split_off(..8)?;
                }
                2 => {
                    let len = usize::try_from(varint(&mut body)?).ok()?;
                    let payload = body.split_off(..len)?;
                    if depth + 1 < path.len() {
                        let mut nested = *self;
                        if nested.fields(payload, path, depth + 1, now).is_some() {
                            *self = nested;
                        }
                    }
                }
                5 => {
                    let bytes = body.split_off(..4)?.try_into().ok()?;
                    let value = f64::from(f32::from_le_bytes(bytes));
                    self.has_fixed32 = true;
                    if field == 1
                        && (0.0..=100.0).contains(&value)
                        && self.percent.is_none_or(|(previous, _)| depth < previous)
                    {
                        self.percent = Some((depth, value));
                    }
                }
                _ => return None,
            }
        }
        Some(())
    }
}

fn varint(body: &mut &[u8]) -> Option<u64> {
    let mut value = 0;
    for shift in (0..64).step_by(7) {
        let byte = *body.split_off_first()?;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}
