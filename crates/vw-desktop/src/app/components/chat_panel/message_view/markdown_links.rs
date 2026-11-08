//! Link targets for assistant Markdown.

use std::path::{Path, PathBuf};

use reqwest::Url;

use crate::app::Message;
use crate::app::message::ViewMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LinkTarget {
    Web(String),
    File(String),
}

impl LinkTarget {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Web(target) | Self::File(target) => target.len(),
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

    pub(super) fn code_path(text: &str) -> Option<Self> {
        let path = text.trim();
        let first_component = path.split('/').next().unwrap_or_default();
        if !path.contains('/') || first_component.chars().any(char::is_whitespace) {
            return None;
        }
        Self::parse(path).filter(|target| matches!(target, Self::File(_)))
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
            let absolute = if let Some(relative) = path.strip_prefix("~/") {
                directories::UserDirs::new().map(|dirs| dirs.home_dir().join(relative))
            } else if Path::new(&path).is_absolute() {
                Some(PathBuf::from(&path))
            } else {
                workspace.map(|root| Path::new(root).join(&path))
            };
            // The existing file-manager endpoint decodes file URLs. Serialize here so literal
            // percent signs, whitespace, and Unicode survive the gateway round trip.
            if let Some(url) = absolute.and_then(|path| Url::from_file_path(path).ok()) {
                Message::View(ViewMessage::OpenPathInFinder(url.into()))
            } else {
                Message::View(ViewMessage::PathRevealFinished {
                    path,
                    result: Err("无法确定本地路径的绝对位置".to_owned()),
                })
            }
        }
    }
}
