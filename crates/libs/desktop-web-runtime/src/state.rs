//! Docked browser input, error and focus state.

#[derive(Debug)]
pub struct WorkBrowserState {
    pub address: String,
    pub error: Option<String>,
    pub window_focused: bool,
    pub closed: bool,
}

impl Default for WorkBrowserState {
    fn default() -> Self {
        Self { address: String::new(), error: None, window_focused: true, closed: true }
    }
}
