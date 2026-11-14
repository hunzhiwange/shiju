//! Content search on the authorized executor; no persistent content index.
use std::{io::Read, path::Path, time::UNIX_EPOCH};

use crate::{
    apps::lifecycle::operations::Operation,
    workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory},
};
use vw_api_types::workspace_memory::{DailyMemoryFilename, DailyMemorySearchResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchError {
    Path(PathError),
    Cancelled,
}
impl From<PathError> for SearchError {
    fn from(value: PathError) -> Self {
        Self::Path(value)
    }
}
impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(error) => error.fmt(f),
            Self::Cancelled => f.write_str("daily memory search cancelled"),
        }
    }
}
impl std::error::Error for SearchError {}

fn checkpoint(operation: &Operation) -> Result<(), SearchError> {
    if operation.is_cancelled() { Err(SearchError::Cancelled) } else { Ok(()) }
}

/// The caller authorizes content reading and owns the scoped operation handle.
/// Run on a blocking worker. Cancellation discards partial results; the caller
/// owns terminal task status, as with other host-owned operations.
pub fn search_daily_memory_files(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
    query: &str,
    operation: &Operation,
) -> Result<Vec<DailyMemorySearchResult>, SearchError> {
    checkpoint(operation)?;
    let directory = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawMemory,
        false,
    )?;
    let Some(directory) = directory else { return Ok(Vec::new()) };
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let query = query.to_lowercase();
    let mut results = Vec::new();
    for name in directory.filenames()? {
        checkpoint(operation)?;
        let Ok(filename) = name.parse::<DailyMemoryFilename>() else { continue };
        let Some(mut file) = directory.regular_file(&name)? else { continue };
        let metadata = file.metadata().map_err(|_| PathError::Io)?;
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            checkpoint(operation)?;
            let n = file.read(&mut chunk).map_err(|_| PathError::Io)?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
        }
        let content = String::from_utf8(bytes).map_err(|_| PathError::Io)?;
        let lower = content.to_lowercase();
        checkpoint(operation)?;
        let mut first = None;
        let mut count = 0;
        for (offset, _) in lower.match_indices(&query) {
            checkpoint(operation)?;
            first.get_or_insert(offset);
            count += 1;
        }
        if first.is_none() && !filename.date().contains(&query) {
            continue;
        }
        // Map folded byte offsets to original character positions. Lowercasing
        // may expand a character (İ -> i + combining dot); never slice original
        // text with an offset from the folded text. Contextual sigma has the
        // same encoded length as its non-contextual lowercase counterpart.
        let mut position = 0;
        if let Some(first) = first {
            let mut folded_offset = 0;
            for (index, ch) in content.chars().enumerate() {
                checkpoint(operation)?;
                let width: usize = ch.to_lowercase().map(char::len_utf8).sum();
                if folded_offset + width > first {
                    position = index;
                    break;
                }
                folded_offset += width;
            }
        }
        let start = position.saturating_sub(50);
        let end = if first.is_some() { position.saturating_add(70) } else { 120 };
        let mut snippet = String::new();
        if start > 0 {
            snippet.push_str("...");
        }
        for (index, ch) in content.chars().enumerate() {
            checkpoint(operation)?;
            if index < start {
                continue;
            }
            if index >= end {
                snippet.push_str("...");
                break;
            }
            snippet.push(ch);
        }
        results.push(DailyMemorySearchResult {
            date: filename.date().to_owned(),
            filename,
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .map_err(|_| PathError::Io)?
                .duration_since(UNIX_EPOCH)
                .map(|time| time.as_secs())
                .unwrap_or(0),
            snippet,
            match_count: count,
        });
    }
    results.sort_by(|a, b| b.filename.as_str().cmp(a.filename.as_str()));
    checkpoint(operation)?;
    Ok(results)
}
