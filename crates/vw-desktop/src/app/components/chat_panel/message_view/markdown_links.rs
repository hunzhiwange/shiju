//! Workspace lookup and cross-domain link effect mapping.
use crate::app::{App, Message, message::ViewMessage};
use desktop_chat_view::message_view::workspace_links as local;
pub(super) use local::LinkTarget;
use std::borrow::Cow;
#[derive(Clone, Default)]
pub(super) struct WorkspaceLinks<'a>(local::WorkspaceLinks<'a>);
impl<'a> WorkspaceLinks<'a> {
    pub(super) fn new(app: &'a App) -> Self {
        let project = app.navigation_tabs.project_path.as_deref();
        let directory = app.session_state.active_session_id
            .as_deref()
            .and_then(|id| app.known_session_directory(id))
            .filter(|path| !path.is_empty())
            .map(Cow::Owned)
            .or_else(|| project.map(Cow::Borrowed));
        let index = directory
            .as_deref()
            .and_then(|path| app.workspace_state.file_tree_model_cache.get_key_value(path))
            .or_else(|| app.workspace_state.file_tree_model_cache.get_key_value(project?))
            .map(|(root, tree)| (root.as_str(), tree));
        Self(local::WorkspaceLinks::new(directory, index))
    }

    pub(super) fn visible_target(&self, target: &LinkTarget) -> Option<LinkTarget> {
        self.0.visible_target(target)
    }
    pub(super) fn open(&self, target: LinkTarget) -> Message {
        map(self.0.open(target))
    }
}
fn map(effect: local::LinkEffect) -> Message {
    match effect {
        local::LinkEffect::OpenUrlExternal(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
        local::LinkEffect::OpenChatPath(path) => Message::View(ViewMessage::OpenChatPath(path)),
        local::LinkEffect::Unresolved(path) => Message::View(ViewMessage::ChatPathKindResolved {
            path,
            result: Err("无法确定本地路径的绝对位置".to_owned()),
        }),
        local::LinkEffect::None => Message::None,
    }
}
