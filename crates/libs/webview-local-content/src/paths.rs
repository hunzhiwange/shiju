use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum StartTarget {
    Url(String),
    LocalDir { root: PathBuf, entry: String },
}

pub fn normalize_url(s: &str) -> String {
    let mut t = s.trim();
    loop {
        let before = t;
        for q in ['`', '"', '\''] {
            if t.len() >= 2 && t.starts_with(q) && t.ends_with(q) {
                t = &t[1..t.len() - 1];
                t = t.trim();
            }
        }
        if t == before {
            break;
        }
    }
    t.to_string()
}

pub fn percent_encode_url_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 8);
    for b in path.as_bytes() {
        let c = *b as char;
        let is_unreserved = matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~');
        if is_unreserved || c == '/' {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(crate) fn percent_decode(s: &str) -> String {
    fn from_hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h1), Some(h2)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2]))
        {
            out.push((h1 << 4) | h2);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    let s = url.strip_prefix("file://")?;
    let s = s.strip_prefix("localhost").unwrap_or(s);
    let s = s.strip_prefix('/').unwrap_or(s);
    let decoded = percent_decode(s);

    #[cfg(target_os = "windows")]
    {
        Some(PathBuf::from(decoded.replace('/', "\\")))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Some(PathBuf::from(format!("/{decoded}")))
    }
}

pub fn resolve_target(target: &str) -> StartTarget {
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return StartTarget::Url(target.to_string());
    }

    let as_path = if lower.starts_with("file://") {
        file_url_to_path(target).unwrap_or_else(|| PathBuf::from(target))
    } else {
        PathBuf::from(target)
    };

    let candidate = match std::fs::metadata(&as_path) {
        Ok(m) if m.is_dir() => {
            let index = as_path.join("index.html");
            if index.is_file() { Some((as_path, index)) } else { None }
        }
        Ok(m) if m.is_file() => as_path.parent().map(|root| (root.to_path_buf(), as_path.clone())),
        _ => None,
    };

    if let Some((root, entry_file)) = candidate
        && let (Ok(root), Ok(entry_file)) =
            (std::fs::canonicalize(&root), std::fs::canonicalize(&entry_file))
        && let Some(name) = entry_file.file_name().and_then(|s| s.to_str())
    {
        return StartTarget::LocalDir { root, entry: format!("/{name}") };
    }

    StartTarget::Url(target.to_string())
}

#[cfg(test)]
mod paths_tests;
