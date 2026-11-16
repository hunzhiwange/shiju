use desktop_file_tree::model::FileTreeNode;
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub use super::markdown_links::LinkTarget;
#[derive(Clone, Default)]
pub struct WorkspaceLinks<'a> {
    directory: Option<Cow<'a, str>>,
    index: Option<(&'a str, &'a FileTreeNode)>,
}

impl<'a> WorkspaceLinks<'a> {
    pub fn new(
        directory: Option<Cow<'a, str>>,
        index: Option<(&'a str, &'a FileTreeNode)>,
    ) -> Self {
        Self { directory, index }
    }

    pub fn visible_target(&self, target: &LinkTarget) -> Option<LinkTarget> {
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

    pub fn open(&self, target: LinkTarget) -> LinkEffect {
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

pub fn link_click_message(target: LinkTarget, workspace: Option<&str>) -> LinkEffect {
    match target {
        LinkTarget::Web(url) => LinkEffect::OpenUrlExternal(url),
        LinkTarget::File(path) => {
            if let Some(absolute) = absolute_local_path(&path, workspace)
                && let Ok(absolute) = absolute.into_os_string().into_string()
            {
                LinkEffect::OpenChatPath(absolute)
            } else {
                LinkEffect::Unresolved(path)
            }
        }
        LinkTarget::InferredFile(_) => LinkEffect::None,
    }
}

#[derive(Debug, Clone)]
pub enum LinkEffect {
    OpenUrlExternal(String),
    OpenChatPath(String),
    Unresolved(String),
    None,
}
