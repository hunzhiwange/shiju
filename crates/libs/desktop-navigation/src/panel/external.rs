//! External target selection; native opening/reveal/clipboard remain host ports.
use crate::components::top_bar::types::ExternalOpenApp;
use std::collections::HashMap;
pub struct State {
    pub open_external_app: ExternalOpenApp,
    pub open_external_exists: HashMap<ExternalOpenApp, bool>,
}
pub struct OpenProject {
    pub path: String,
    pub target: ExternalOpenApp,
}
impl State {
    pub fn available(&self, target: ExternalOpenApp) -> bool {
        self.open_external_exists.get(&target).copied().unwrap_or(false)
    }
    pub fn preferred(
        &self,
        panels: &mut super::settings::State,
        path: Option<&str>,
    ) -> Option<OpenProject> {
        panels.close_menu();
        let path = path?;
        self.available(self.open_external_app)
            .then(|| OpenProject { path: path.into(), target: self.open_external_app })
    }
    pub fn select(
        &mut self,
        panels: &mut super::settings::State,
        path: Option<&str>,
        target: ExternalOpenApp,
    ) -> Option<OpenProject> {
        self.open_external_app = target;
        self.preferred(panels, path)
    }
}
