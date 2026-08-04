use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiEffect {
    OpenWorkspaceFile { path: PathBuf, line: Option<usize> },
    LocateTranscriptMessage { message_id: String },
    RestoreHistory { value: String },
}
