//! Bounded decoded output. Cursors count observed chunks, independently of readers.
use anyhow::bail;
use std::collections::VecDeque;

pub(super) const OUTPUT_LIMIT: usize = 1024 * 1024;
#[derive(Default)]
pub(super) struct OutputBuffer {
    chunks: VecDeque<(u64, String)>,
    next: u64,
    bytes: usize,
    dropped: u64,
}
pub(super) struct OutputRead {
    pub output: String,
    pub next_cursor: u64,
    pub truncated: bool,
    pub dropped_bytes: u64,
}
impl OutputBuffer {
    pub(super) fn push(&mut self, bytes: &[u8]) {
        // The decoder supplies valid UTF-8; chunking also bounds individual entries.
        let text = String::from_utf8_lossy(bytes);
        let mut remaining = text.as_ref();
        while !remaining.is_empty() {
            let mut end = remaining.len().min(8192);
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            let chunk = remaining[..end].to_owned();
            remaining = &remaining[end..];
            self.bytes += chunk.len();
            self.chunks.push_back((self.next, chunk));
            self.next += 1;
            while self.bytes > OUTPUT_LIMIT {
                let (_, removed) = self.chunks.pop_front().expect("nonempty output");
                self.bytes -= removed.len();
                self.dropped += removed.len() as u64;
            }
        }
    }
    pub(super) fn read(&self, cursor: u64) -> anyhow::Result<OutputRead> {
        if cursor > self.next {
            bail!("Shell output cursor is ahead of output");
        }
        let first = self.chunks.front().map_or(self.next, |(id, _)| *id);
        let truncated = cursor < first;
        let output = self
            .chunks
            .iter()
            .filter(|(id, _)| *id >= cursor)
            .map(|(_, text)| text.as_str())
            .collect();
        Ok(OutputRead {
            output,
            next_cursor: self.next,
            truncated,
            dropped_bytes: if truncated { self.dropped } else { 0 },
        })
    }
    #[cfg(test)]
    pub(super) fn retained_bytes(&self) -> usize {
        self.bytes
    }
}

#[derive(Default)]
pub(super) struct OutputDecoder {
    pending: Vec<u8>,
}
impl OutputDecoder {
    pub(super) fn push(&mut self, bytes: &[u8], eof: bool, output: &mut OutputBuffer) {
        self.pending.extend_from_slice(bytes);
        let mut consumed = 0;
        while consumed < self.pending.len() {
            match std::str::from_utf8(&self.pending[consumed..]) {
                Ok(text) => {
                    output.push(text.as_bytes());
                    consumed = self.pending.len();
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    output.push(&self.pending[consumed..consumed + valid]);
                    consumed += valid;
                    match error.error_len() {
                        Some(length) => {
                            output.push("�".as_bytes());
                            consumed += length;
                        }
                        None if eof => {
                            output.push("�".as_bytes());
                            consumed = self.pending.len();
                        }
                        None => break,
                    }
                }
            }
        }
        self.pending.drain(..consumed);
    }
}
