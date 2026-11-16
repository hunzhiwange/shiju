pub use crate::message_view::workspace_links::LinkTarget;
use std::borrow::Cow;
#[derive(Clone, Default)]
pub struct WorkspaceLinks<'a>(crate::message_view::workspace_links::WorkspaceLinks<'a>);
impl<'a> WorkspaceLinks<'a> {
    pub fn new(app: &'a super::fixture::RenderFixture) -> Self {
        Self(crate::message_view::workspace_links::WorkspaceLinks::new(
            app.project_path.as_deref().map(Cow::Borrowed),
            app.project_path
                .as_ref()
                .and_then(|p| app.file_tree_model_cache.get_key_value(p))
                .map(|(p, t)| (p.as_str(), t)),
        ))
    }
    pub fn visible_target(&self, t: &LinkTarget) -> Option<LinkTarget> {
        self.0.visible_target(t)
    }
    pub fn open(&self, t: LinkTarget) -> super::fixture::Message {
        super::fixture::Message::View(self.0.open(t))
    }
    pub fn input(
        &self,
    ) -> crate::message_view::markdown_body::WorkspaceLinks<'a, super::fixture::Message> {
        let visible = self.clone();
        let open = self.clone();
        crate::message_view::markdown_body::WorkspaceLinks {
            visible: std::rc::Rc::new(move |t| visible.visible_target(t)),
            open: std::rc::Rc::new(move |t| open.open(t)),
        }
    }
}
pub fn link_click_message(t: LinkTarget, w: Option<&str>) -> super::fixture::Message {
    super::fixture::Message::View(crate::message_view::workspace_links::link_click_message(t, w))
}
