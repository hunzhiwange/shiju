//! Registered source Chat paths for the resident listener only.
pub(crate) fn is_path(path: &str) -> bool {
    matches!(
        path,
        "/chat/completions"
            | "/v1/chat/completions"
            | "/v1/v1/chat/completions"
            | "/codex/v1/chat/completions"
    )
}
