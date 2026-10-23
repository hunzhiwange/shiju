//! Bounded, UTF-8-safe rolling summaries shared by the session entry points.

use crate::app::agent::provider::provider::ModelLimit;

pub(crate) struct SummaryChunks<'a> {
    remaining: &'a str,
    summary: String,
    chunk_bytes: usize,
}

impl<'a> SummaryChunks<'a> {
    pub(crate) fn new(transcript: &'a str, limit: Option<&ModelLimit>) -> Result<Self, String> {
        // A byte budget is deliberately conservative for multilingual text. Reserve half
        // for the previous summary and leave room for instructions and protocol overhead.
        let window = limit
            .filter(|limit| limit.context > 0 || limit.input.is_some())
            .map(super::processor::context_governance::effective_input_window)
            .unwrap_or(32_000)
            .min(32_000) as usize;
        let chunk_bytes = window.saturating_sub(512) / 2;
        if chunk_bytes < 256 {
            return Err("model input window is too small for internal compaction".to_string());
        }
        Ok(Self { remaining: transcript, summary: String::new(), chunk_bytes })
    }

    pub(crate) fn next_prompt(&mut self) -> Option<String> {
        if self.remaining.is_empty() {
            return None;
        }
        let mut end = self.remaining.len().min(self.chunk_bytes);
        while !self.remaining.is_char_boundary(end) {
            end -= 1;
        }
        let (chunk, rest) = self.remaining.split_at(end);
        self.remaining = rest;
        Some(super::compaction::internal_summary_prompt(&format!(
            "Merge the previous summary with the next conversation fragment. Return a concise summary of at most {} characters.\n\nPrevious summary:\n{}\n\nNext conversation fragment:\n{}",
            self.chunk_bytes / 4,
            self.summary,
            chunk,
        )))
    }

    pub(crate) fn accept(&mut self, summary: String) -> Result<(), String> {
        let summary = summary.trim();
        if summary.is_empty() {
            return Err("internal compaction generated an empty summary".to_string());
        }
        if summary.len() > self.chunk_bytes {
            return Err("internal compaction summary exceeds its input budget".to_string());
        }
        self.summary = summary.to_string();
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<String, String> {
        if !self.remaining.is_empty() || self.summary.is_empty() {
            return Err("internal compaction did not summarize all history".to_string());
        }
        Ok(self.summary)
    }
}

#[cfg(test)]
#[path = "compaction_summary_tests.rs"]
mod tests;
