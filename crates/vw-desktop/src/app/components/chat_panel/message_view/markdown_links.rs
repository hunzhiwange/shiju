//! Link targets for assistant Markdown.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use reqwest::Url;

use crate::app::components::file_tree::model::FileTreeNode;
use crate::app::message::ViewMessage;
use crate::app::{App, Message};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LinkTarget {
    Web(String),
    File(String),
    InferredFile(String),
}

impl LinkTarget {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Web(target) | Self::File(target) | Self::InferredFile(target) => target.len(),
        }
    }

    pub(super) fn parse(target: &str) -> Option<Self> {
        if Path::new(target).is_absolute() {
            return Self::local_path(target);
        }
        let Ok(url) = Url::parse(target) else {
            return Self::local_path(target);
        };
        match url.scheme() {
            "http" | "https"
                if url.has_host() && url.username().is_empty() && url.password().is_none() =>
            {
                Some(Self::Web(url.into()))
            }
            "file"
                if target.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
                    && url.host_str().is_none()
                    && url.username().is_empty() =>
            {
                let path = url.to_file_path().ok()?.into_os_string().into_string().ok()?;
                valid_path(&path).then_some(Self::File(path))
            }
            _ => None,
        }
    }

    pub(super) fn markdown_target(target: &str) -> Option<Self> {
        let link = Self::parse(target)?;
        match link {
            Self::File(path)
                if !target.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:")) =>
            {
                let decoded = urlencoding::decode(&path).ok()?.into_owned();
                valid_path(&decoded).then_some(Self::File(decoded))
            }
            link => Some(link),
        }
    }

    pub(super) fn code_target(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some((scheme, _)) = text.split_once("://")
            && ["http", "https", "file"].iter().any(|value| scheme.eq_ignore_ascii_case(value))
        {
            if text.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
                return None;
            }
            return Self::parse(text);
        }
        Self::inferred_file_path(text)
    }

    pub(super) fn inferred_file_path(path: &str) -> Option<Self> {
        if !path.contains('/') && Path::new(path).extension().is_none() {
            return None;
        }
        match Self::local_path(path) {
            Some(Self::File(path)) => Some(Self::InferredFile(path)),
            _ => None,
        }
    }

    pub(super) fn local_path(path: &str) -> Option<Self> {
        let path = strip_source_location(path);
        let candidate = Path::new(path);
        if valid_path(path)
            && (candidate.is_absolute() || (!path.contains(':') && !path.starts_with(['#', '?'])))
        {
            Some(Self::File(path.to_owned()))
        } else {
            None
        }
    }
}

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

fn valid_path(path: &str) -> bool {
    !path.trim().is_empty()
        && !path.starts_with("//")
        && !path.starts_with("\\\\")
        && !path.chars().any(char::is_control)
}

fn strip_source_location(path: &str) -> &str {
    if let Some((file, anchor)) = path.rsplit_once('#')
        && (anchor.strip_prefix("line-").is_some_and(line_number)
            || anchor.strip_prefix('L').is_some_and(|range| {
                range.split_once("-L").map_or_else(
                    || line_number(range),
                    |(start, end)| line_number(start) && line_number(end),
                )
            }))
    {
        return file;
    }

    let Some((file, line)) = path.rsplit_once(':') else { return path };
    if !line_number(line) {
        return path;
    }
    if let Some((base, line)) = file.rsplit_once(':')
        && line_number(line)
    {
        return base;
    }
    file
}

fn line_number(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
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
