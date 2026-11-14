//! Source-registered Messages paths for the resident Claude listener only.
//! Desktop retains its separate application credential and namespace.
pub(crate) fn is_path(path: &str) -> bool {
    matches!(path, "/v1/messages" | "/claude/v1/messages")
}
