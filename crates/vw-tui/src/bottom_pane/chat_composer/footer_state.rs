//! 本模块集中实现 `footer_state` 在底部面板中的职责。


//! VibeWindow 底部使用的本地、时间无关的状态
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
