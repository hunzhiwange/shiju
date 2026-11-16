//! Bounded, case-sensitive literal search of loaded logical lines.
use ratatui::text::Line;

pub(super) const SCAN_BYTES: usize = 16 * 1024;
const QUERY_BYTES: usize = 4096;

#[derive(Debug, Default)]
pub(super) struct Search {
    pub query: String,
    pub editing: bool,
    pub saved: (usize, bool),
    pub hits: Vec<(usize, usize)>,
    pub selected: usize,
    pub line: usize,
    pub(super) span: usize,
    pub(super) byte: usize,
    pub(super) offset: usize,
    pub(super) matched: usize,
    pub(super) prefix: Vec<usize>,
}

impl Search {
    pub fn push(&mut self, ch: char) {
        if self.query.len() + ch.len_utf8() <= QUERY_BYTES {
            self.query.push(ch);
            self.restart();
        }
    }

    pub fn restart(&mut self) {
        self.hits.clear();
        self.selected = 0;
        self.line = 0;
        self.span = 0;
        self.byte = 0;
        self.offset = 0;
        self.matched = 0;
        self.prefix = vec![0; self.query.len()];
        let bytes = self.query.as_bytes();
        for i in 1..bytes.len() {
            let mut j = self.prefix[i - 1];
            while j > 0 && bytes[i] != bytes[j] {
                j = self.prefix[j - 1];
            }
            if bytes[i] == bytes[j] {
                j += 1;
            }
            self.prefix[i] = j;
        }
    }

    /// Each byte and each empty span/line consumes one unit of the frame budget.
    pub fn scan(&mut self, lines: &[Line<'static>], budget: usize) -> usize {
        if self.query.is_empty() {
            return 0;
        }
        let query = self.query.as_bytes();
        let mut spent = 0;
        while spent < budget && self.line < lines.len() {
            spent += 1;
            let Some(span) = lines[self.line].spans.get(self.span) else {
                self.line += 1;
                self.span = 0;
                self.byte = 0;
                self.offset = 0;
                self.matched = 0;
                continue;
            };
            let Some(&byte) = span.content.as_bytes().get(self.byte) else {
                self.span += 1;
                self.byte = 0;
                continue;
            };
            while self.matched > 0 && byte != query[self.matched] {
                self.matched = self.prefix[self.matched - 1];
            }
            if byte == query[self.matched] {
                self.matched += 1;
            }
            self.byte += 1;
            self.offset += 1;
            if self.matched == query.len() {
                self.hits.push((self.line, self.offset - query.len()));
                self.matched = self.prefix[self.matched - 1];
            }
        }
        spent
    }

    pub fn invalidate_from(&mut self, line: usize) {
        self.hits.retain(|hit| hit.0 < line);
        self.selected = self.selected.min(self.hits.len().saturating_sub(1));
        if self.line >= line {
            self.line = line;
            self.span = 0;
            self.byte = 0;
            self.offset = 0;
            self.matched = 0;
        }
    }

    pub fn navigate(&mut self, backwards: bool) {
        if !self.hits.is_empty() {
            self.selected = if backwards {
                (self.selected + self.hits.len() - 1) % self.hits.len()
            } else {
                (self.selected + 1) % self.hits.len()
            };
        }
    }
}
