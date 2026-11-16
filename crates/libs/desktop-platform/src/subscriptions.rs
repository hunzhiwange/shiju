//! Subscription policy; the host owns Iced recipes and projects domain activity.
#[derive(Debug, Default, Clone, Copy)]
pub struct Activity {
    pub startup_transition_active: bool,
    pub update_busy: bool,
    pub conversation_requesting: bool,
    pub explore_animating: bool,
    pub companion_visible: bool,
    pub companion_animating: bool,
    pub diagnostic_active: bool,
}
impl Activity {
    pub fn needs_tick(self) -> bool {
        self.startup_transition_active
            || self.update_busy
            || self.conversation_requesting
            || self.explore_animating
            || (self.companion_visible && self.companion_animating)
            || self.diagnostic_active
    }
}
