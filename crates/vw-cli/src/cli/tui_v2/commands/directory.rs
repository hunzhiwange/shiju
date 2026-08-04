use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectoryCommand {
    Change(PathBuf),
    Project,
}
