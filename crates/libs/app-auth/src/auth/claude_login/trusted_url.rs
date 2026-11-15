use super::*;

pub(super) fn printed_url(line: &[u8]) -> Option<String> {
    let line = String::from_utf8_lossy(line);
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.next_if_eq(&'[').is_some() {
            // Skip the ANSI CSI parameters and final byte.
            chars.by_ref().find(|ch| ('@'..='~').contains(ch));
        } else {
            plain.push(ch);
        }
    }
    plain.split_ascii_whitespace().find_map(|part| trusted_url(part.as_bytes()))
}

pub(super) fn trusted_url(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.len() > OUTPUT_LINE_LIMIT
        || text.chars().any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return None;
    }
    let url = Url::parse(text).ok()?;
    if !text.starts_with("https://")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || !matches!(
            (url.host_str()?, url.path()),
            ("claude.com", "/cai/oauth/authorize")
                | ("claude.ai" | "platform.claude.com", "/oauth/authorize")
        )
    {
        return None;
    }

    let mut seen = HashSet::new();
    for (key, value) in url.query_pairs() {
        // Only the official CLI's browser challenge fields may leave the host.
        let allowed = match key.as_ref() {
            "redirect_uri" => trusted_redirect(&value),
            "code" => value == "true",
            "response_type" => value == "code",
            "code_challenge" => {
                value.len() == 43
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            }
            "code_challenge_method" => value == "S256",
            "orgUUID" => uuid::Uuid::parse_str(&value).is_ok(),
            "client_id" | "scope" | "state" => !value.is_empty(),
            _ => false,
        };
        if !allowed || !seen.insert(key) {
            return None;
        }
    }
    // Without S256, the challenge can be the verifier itself.
    if seen.contains("code_challenge") != seen.contains("code_challenge_method") {
        return None;
    }
    Some(url.into())
}

pub(super) fn trusted_redirect(text: &str) -> bool {
    if text.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return false;
    }
    let Ok(url) = Url::parse(text) else { return false };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return false;
    }
    match (url.scheme(), url.host_str(), url.port()) {
        ("http", _, Some(port)) => loopback_host(&url) && port != 0,
        ("https", Some("platform.claude.com" | "console.anthropic.com"), None) => {
            url.path() == "/oauth/code/callback"
        }
        _ => false,
    }
}

pub(super) fn loopback_host(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
}
