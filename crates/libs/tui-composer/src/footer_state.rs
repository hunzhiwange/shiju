//! 本模块集中实现 `footer_state` 在底部面板中的职责。

//! VibeWindow 底部使用的本地、时间无关的状态
//! Local, time-independent state used by the VibeWindow footer.

use super::footer::FooterMode;

#[derive(Debug, Default)]
pub struct FooterState {
    mode: FooterMode,
    shortcuts_visible: bool,
}

impl FooterState {
    pub fn new() -> Self {
        Self { mode: FooterMode::ComposerEmpty, shortcuts_visible: false }
    }

    pub fn mode(&self) -> FooterMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: FooterMode) {
        self.mode = mode;
    }

    pub fn shortcuts_visible(&self) -> bool {
        self.shortcuts_visible
    }

    pub fn set_shortcuts_visible(&mut self, visible: bool) {
        self.shortcuts_visible = visible;
    }
}
