//! Vim editing mode persisted in TUI prompt/session options.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VimMode {
    #[default]
    Disabled,
    Normal,
    Insert,
}
