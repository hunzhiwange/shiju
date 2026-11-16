use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use crate::app::components::file_tree::model::FileTreeNode;
use crate::app::message::ViewMessage;
use crate::app::{App, Message};
pub(super) use desktop_chat_view::message_view::markdown_links::LinkTarget;
#[derive(Clone, Default)]
pub(super) struct WorkspaceLinks<'a> {
    directory: Option<Cow<'a, str>>,
    index: Option<(&'a str, &'a FileTreeNode)>,
}

impl<'a> WorkspaceLinks<'a> {
    pub(super) fn new(app: &'a App) -> Self {
        let project = app.project_path.as_deref();
        let directory = app
            .active_session_id
            .as_deref()
            .and_then(|id| app.known_session_directory(id))
            .filter(|path| !path.is_empty())
            .map(Cow::Owned)
            .or_else(|| project.map(Cow::Borrowed));
        let index = directory
            .as_deref()
            .and_then(|path| app.file_tree_model_cache.get_key_value(path))
            .or_else(|| app.file_tree_model_cache.get_key_value(project?))
            .map(|(root, tree)| (root.as_str(), tree));
        Self { directory, index }
    }

    pub(super) fn visible_target(&self, target: &LinkTarget) -> Option<LinkTarget> {
        let LinkTarget::InferredFile(path) = target else { return Some(target.clone()) };
        let (root, tree) = self.index?;
        let absolute = absolute_local_path(path, self.directory.as_deref())?;
        let relative = absolute.strip_prefix(root).ok()?;
        if indexed_path(tree, relative) {
            Some(LinkTarget::File(absolute.into_os_string().into_string().ok()?))
        } else {
            None
        }
    }

    pub(super) fn open(&self, target: LinkTarget) -> Message {
        link_click_message(target, self.directory.as_deref())
    }
}

fn indexed_path(mut tree: &FileTreeNode, relative: &Path) -> bool {
    let mut components = relative.components().peekable();
    let mut key = String::new();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else { return false };
        let Some(name) = name.to_str() else { return false };
        if !key.is_empty() {
            key.push('/');
        }
        key.push_str(name);
        match tree.children.get(name) {
            Some(child) => tree = child,
            None => {
                return components.peek().is_none() && tree.files.iter().any(|file| *file == key);
            }
        }
    }
    true
}

fn absolute_local_path(path: &str, workspace: Option<&str>) -> Option<PathBuf> {
    if let Some(relative) = path.strip_prefix("~/") {
        directories::UserDirs::new().map(|dirs| dirs.home_dir().join(relative))
    } else if Path::new(path).is_absolute() {
        Some(PathBuf::from(path))
    } else {
        workspace.map(|root| Path::new(root).join(path))
    }
    .filter(|path| path.is_absolute())
}

pub(super) fn link_click_message(target: LinkTarget, workspace: Option<&str>) -> Message {
    match target {
        LinkTarget::Web(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
        LinkTarget::File(path) => {
            if let Some(absolute) = absolute_local_path(&path, workspace)
                && let Ok(absolute) = absolute.into_os_string().into_string()
            {
                Message::View(ViewMessage::OpenChatPath(absolute))
            } else {
                Message::View(ViewMessage::ChatPathKindResolved {
                    path,
                    result: Err("无法确定本地路径的绝对位置".to_owned()),
                })
            }
        }
        LinkTarget::InferredFile(_) => Message::None,
    }
}
