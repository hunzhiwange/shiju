//! Pure file-change data consumed by the copied diff/history cells.

use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileChange {
    Add { content: String },
    Delete { content: String },
    Update { unified_diff: String, move_path: Option<PathBuf> },
}

impl FileChange {
    pub fn line_counts(&self) -> (usize, usize) {
        match self {
            Self::Add { content } => (content.lines().count(), 0),
            Self::Delete { content } => (0, content.lines().count()),
            Self::Update { unified_diff, .. } => unified_diff
                .lines()
                .filter(|line| !line.starts_with("+++") && !line.starts_with("---"))
                .fold((0, 0), |(added, removed), line| {
                    if line.starts_with('+') {
                        (added + 1, removed)
                    } else if line.starts_with('-') {
                        (added, removed + 1)
                    } else {
                        (added, removed)
                    }
                }),
        }
    }
}
