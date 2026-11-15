//! Link targets for assistant Markdown.

use std::path::{Path};

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Web(String),
    File(String),
    InferredFile(String),
}

impl LinkTarget {
    pub fn len(&self) -> usize {
        match self {
            Self::Web(target) | Self::File(target) | Self::InferredFile(target) => target.len(),
        }
    }

    pub fn parse(target: &str) -> Option<Self> {
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
                #[cfg(not(target_arch = "wasm32"))]
                let path = url.to_file_path().ok()?.into_os_string().into_string().ok()?;
                #[cfg(target_arch = "wasm32")]
                let path = {
                    let path = urlencoding::decode(url.path()).ok()?.into_owned();
                    // A Windows drive URL is /C:/..., while gateway paths use C:/....
                    if path.as_bytes().get(2) == Some(&b':')
                        && path.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic)
                    {
                        path[1..].to_string()
                    } else {
                        path
                    }
                };
                valid_path(&path).then_some(Self::File(path))
            }
            _ => None,
        }
    }

    pub fn markdown_target(target: &str) -> Option<Self> {
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

    pub fn code_target(text: &str) -> Option<Self> {
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

    pub fn inferred_file_path(path: &str) -> Option<Self> {
        if !path.contains('/') && Path::new(path).extension().is_none() {
            return None;
        }
        match Self::local_path(path) {
            Some(Self::File(path)) => Some(Self::InferredFile(path)),
            _ => None,
        }
    }

    pub fn local_path(path: &str) -> Option<Self> {
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
