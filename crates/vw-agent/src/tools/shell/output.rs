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

pub(super) struct OutputDecoder {
    pending: Vec<u8>,
    encoding: Option<StreamEncoding>,
}

impl Default for OutputDecoder {
    fn default() -> Self {
        Self::new(cfg!(windows))
    }
}

#[derive(Clone, Copy)]
enum StreamEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl OutputDecoder {
    pub(super) fn new(detect_windows_encoding: bool) -> Self {
        Self {
            pending: Vec::new(),
            encoding: if detect_windows_encoding { None } else { Some(StreamEncoding::Utf8) },
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8], eof: bool, output: &mut OutputBuffer) {
        self.pending.extend_from_slice(bytes);
        if self.encoding.is_none() {
            // Startup errors from Windows PowerShell can be UTF-16LE without a
            // BOM, before our UTF-8 setup script has had a chance to run.
            if self.pending.len() < 4 && !eof {
                return;
            }
            let (encoding, bom) = if self.pending.starts_with(&[0xff, 0xfe]) {
                (StreamEncoding::Utf16Le, 2)
            } else if self.pending.starts_with(&[0xfe, 0xff]) {
                (StreamEncoding::Utf16Be, 2)
            } else if self.pending.starts_with(&[0xef, 0xbb, 0xbf]) {
                (StreamEncoding::Utf8, 3)
            } else if matches!(self.pending.get(..4), Some([a, 0, b, 0]) if a.is_ascii_graphic() && b.is_ascii_graphic())
            {
                (StreamEncoding::Utf16Le, 0)
            } else if matches!(self.pending.get(..4), Some([0, a, 0, b]) if a.is_ascii_graphic() && b.is_ascii_graphic())
            {
                (StreamEncoding::Utf16Be, 0)
            } else {
                (StreamEncoding::Utf8, 0)
            };
            self.encoding = Some(encoding);
            self.pending.drain(..bom);
        }
        match self.encoding.expect("stream encoding selected") {
            StreamEncoding::Utf8 => self.push_utf8(eof, output),
            StreamEncoding::Utf16Le => self.push_utf16(true, eof, output),
            StreamEncoding::Utf16Be => self.push_utf16(false, eof, output),
        }
    }

    fn push_utf16(&mut self, little_endian: bool, eof: bool, output: &mut OutputBuffer) {
        let mut units: Vec<u16> = self
            .pending
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let bytes = [pair[0], pair[1]];
                if little_endian { u16::from_le_bytes(bytes) } else { u16::from_be_bytes(bytes) }
            })
            .collect();
        // Retain a split surrogate pair as well as any incomplete code unit.
        if !eof && units.last().is_some_and(|unit| (0xd800..=0xdbff).contains(unit)) {
            units.pop();
        }
        let consumed = units.len() * 2;
        output.push(String::from_utf16_lossy(&units).as_bytes());
        self.pending.drain(..consumed);
        if eof && !self.pending.is_empty() {
            output.push("�".as_bytes());
            self.pending.clear();
        }
    }

    fn push_utf8(&mut self, eof: bool, output: &mut OutputBuffer) {
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
