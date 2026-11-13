//! Byte-oriented SSE lines. Invalid UTF-8 is rejected rather than repaired in JSON.
use super::stream_observer::InvalidStream;

#[derive(Default)]
pub(super) struct Lines {
    pending: Vec<u8>,
    started: bool,
}

impl Lines {
    pub(super) fn len(&self) -> usize {
        self.pending.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub(super) fn push(
        &mut self,
        bytes: &[u8],
        limit: usize,
    ) -> Result<Vec<String>, InvalidStream> {
        let mut lines = Vec::new();
        for fragment in bytes.split_inclusive(|byte| *byte == b'\n') {
            if fragment.len() > limit.saturating_sub(self.pending.len()) {
                return Err(InvalidStream::LimitExceeded);
            }
            self.pending.extend_from_slice(fragment);
            if fragment.last() != Some(&b'\n') {
                continue;
            }
            let line = std::str::from_utf8(&self.pending).map_err(|_| InvalidStream::Malformed)?;
            let line = line.strip_suffix('\n').unwrap_or(line);
            let line = line.strip_suffix('\r').unwrap_or(line);
            let line =
                if !self.started { line.strip_prefix('\u{feff}').unwrap_or(line) } else { line };
            if line.contains('\r') {
                return Err(InvalidStream::Malformed);
            }
            lines.push(line.to_owned());
            self.started = true;
            self.pending.clear();
        }
        Ok(lines)
    }
}

/// Resident routing accepts a final SSE block without its blank delimiter.
/// Session relays retain their explicit terminal/delimiter validation policy.
pub(crate) fn resident_body(body: axum::body::Body) -> axum::body::Body {
    use futures_util::StreamExt;
    axum::body::Body::from_stream(futures_util::stream::unfold(
        (body.into_data_stream(), Lines::default(), false),
        |(mut stream, mut lines, done)| async move {
            if done {
                return None;
            }
            loop {
                let next = stream.next().await;
                let eof = next.is_none();
                let parsed = match next {
                    Some(Ok(bytes)) => lines.push(&bytes, 4 * 1024 * 1024),
                    Some(Err(_)) => {
                        return Some((
                            Err(std::io::Error::other("Upstream SSE read failed")),
                            (stream, lines, true),
                        ));
                    }
                    None => lines.push(b"\n\n", 4 * 1024 * 1024),
                };
                let output = match parsed {
                    Ok(parsed) => parsed.into_iter().map(|line| line + "\n").collect::<String>(),
                    Err(_) => {
                        return Some((
                            Err(std::io::Error::other("Invalid upstream SSE framing")),
                            (stream, lines, true),
                        ));
                    }
                };
                if output.is_empty() && !eof {
                    continue;
                }
                return Some((Ok(axum::body::Bytes::from(output)), (stream, lines, eof)));
            }
        },
    ))
}
