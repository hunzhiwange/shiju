use crate::header::HeaderInput;
#[derive(Default)]
pub struct HeaderFixture {
    pub active_session_id: Option<String>,
    pub project_path: Option<String>,
    pub sessions: Vec<session_info::Info>,
    pub project_sessions: std::collections::HashMap<String, Vec<session_info::Info>>,
    pub session_runtime_states:
        std::collections::HashMap<String, desktop_session::session::agent::SessionRuntimeState<()>>,
    pub show_session_actions_popover: bool,
    pub fullscreen: bool,
    pub chat_panel_mode: bool,
}
impl HeaderFixture {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }
    pub fn input(&self) -> HeaderInput<'_> {
        let runtime =
            self.active_session_id.as_ref().and_then(|id| self.session_runtime_states.get(id));
        HeaderInput {
            active_session_id: self.active_session_id.clone(),
            project_path: self.project_path.clone(),
            sessions: &self.sessions,
            project_sessions: &self.project_sessions,
            desktop_locale: Default::default(),
            chat_panel_mode: self.chat_panel_mode,
            fullscreen: self.fullscreen,
            half_fullscreen: false,
            show_session_actions_popover: self.show_session_actions_popover,
            status_animation_frame: 0,
            title: String::new(),
            usage_percent: 0.0,
            usage_details: Default::default(),
            input_limit: 0,
            limit_is_default: false,
            runtime: (
                runtime.is_some_and(|r| r.request_lifecycle.is_requesting),
                runtime.map_or(0, |r| r.request_lifecycle.queue.len()),
                runtime.is_some_and(|r| r.has_unseen_success),
            ),
            now_ms: 0,
            header_background: None,
        }
    }
}
