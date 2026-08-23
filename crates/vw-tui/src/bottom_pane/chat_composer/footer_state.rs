//! Local, time-independent state used by the VibeWindow footer.

use super::footer::FooterMode;

#[derive(Debug, Default)]
pub(super) struct FooterState {
    pub(super) mode: FooterMode,
    pub(super) shortcuts_visible: bool,
}

impl FooterState {
    pub(super) fn new() -> Self {
        Self { mode: FooterMode::ComposerEmpty, shortcuts_visible: false }
    }
}
