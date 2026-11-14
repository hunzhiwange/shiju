use url::Url;

pub fn is_safe_navigation_url(url: &str) -> bool {
    if url.chars().any(|character| character.is_whitespace() || character.is_control()) {
        return false;
    }

    let Some((scheme, remainder)) = url.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }

    let authority =
        remainder.split_once(['/', '?', '#']).map_or(remainder, |(authority, _)| authority);
    authority.rsplit('@').next().is_some_and(|host| !host.is_empty())
}

pub fn is_page_link_url(url: &str) -> bool {
    is_safe_navigation_url(url)
        || (!url.chars().any(char::is_control)
            && url.split_once(':').is_some_and(|(scheme, value)| {
                !value.is_empty()
                    && (scheme.eq_ignore_ascii_case("blob") || scheme.eq_ignore_ascii_case("data"))
            }))
}

pub struct ExternalTarget(Url);

impl ExternalTarget {
    pub fn parse(input: &str) -> Option<Self> {
        // The URL parser can strip raw controls. Reject them and encoded ASCII
        // controls before any normalization, including mail header delimiters.
        if input.chars().any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\') {
            return None;
        }
        for (index, byte) in input.bytes().enumerate() {
            if byte == b'%' {
                let hex = input.get(index + 1..index + 3)?;
                if u8::from_str_radix(hex, 16).ok()?.is_ascii_control() {
                    return None;
                }
            }
        }
        let url = Url::parse(input).ok()?;
        let scheme = url.scheme();
        let payload = input.split_once(':')?.1;
        // Windows can treat a single-letter prefix as a drive-relative path.
        if !(2..=64).contains(&scheme.len())
            || matches!(payload, "" | "//")
            || matches!(
                scheme,
                "http"
                    | "https"
                    | "about"
                    | "blob"
                    | "data"
                    | "file"
                    | "filesystem"
                    | "javascript"
                    | "vbscript"
                    | "view-source"
                    | "devtools"
                    | "resource"
                    | "jar"
                    | "wyciwyg"
                    | "chrome"
                    | "edge"
                    | "opera"
                    | "safari"
                    | "brave"
                    | "vivaldi"
                    | "firefox"
                    | "ftp"
                    | "ws"
                    | "wss"
                    | "vibe"
                    | "vibe-mcp"
                    | "shell"
                    | "search"
                    | "search-ms"
                    | "smb"
                    | "afp"
                    | "nfs"
                    | "x-man-page"
                    | "x-scheme-handler"
                    | "intent"
                    | "powershell"
                    | "cmd"
                    | "mshta"
                    | "msdt"
                    | "x-callback-url"
            )
            || [
                "chrome-",
                "edge-",
                "moz-",
                "safari-",
                "webkit-",
                "ms-",
                "microsoft-",
                "windows-",
                "x-ms-",
                "apple-",
                "x-apple",
                "com.apple.",
            ]
            .iter()
            .any(|prefix| scheme.starts_with(prefix))
        {
            return None;
        }
        Some(Self(url))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }
}

pub enum Navigation {
    InWebView,
    External(ExternalTarget),
    Blocked,
}

pub fn navigation(url: &str, local_site: bool) -> Navigation {
    if is_page_link_url(url)
        || matches!(url, "about:blank" | "about:srcdoc")
        || (local_site && url.starts_with("vibe://localhost/"))
    {
        Navigation::InWebView
    } else {
        ExternalTarget::parse(url).map_or(Navigation::Blocked, Navigation::External)
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
