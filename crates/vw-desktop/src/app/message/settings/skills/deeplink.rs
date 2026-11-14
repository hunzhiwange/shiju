//! Untrusted source links are previews until explicitly confirmed in repository settings.
use vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillRepoEntry;

#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub repo: SkillRepoEntry,
    // Bound by the caller, never taken from a URL supplied filesystem path.
    pub directory: String,
    pub host: String,
}

pub(crate) fn parse(link: &str, directory: String, host: String) -> Result<Request, String> {
    let invalid = || "技能来源链接无效".to_owned();
    if link.len() > 4096 || link.contains('#') { return Err(invalid()); }
    let query = link.trim().strip_prefix("ccswitch://v1/import?").ok_or_else(invalid)?;
    let mut fields = std::collections::BTreeMap::new();
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').ok_or_else(invalid)?;
        let key = urlencoding::decode(key).map_err(|_| invalid())?.into_owned();
        let value = urlencoding::decode(value).map_err(|_| invalid())?.into_owned();
        if !matches!(key.as_str(), "resource" | "repo" | "branch" | "enabled")
            || fields.insert(key, value).is_some() { return Err(invalid()); }
    }
    if fields.get("resource").map(String::as_str) != Some("skill") { return Err(invalid()); }
    let (owner, name) = fields.get("repo").ok_or_else(invalid)?.split_once('/').ok_or_else(invalid)?;
    if owner.is_empty() || owner.len() > 39
        || !owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || name.is_empty() || name.len() > 100 || matches!(name, "." | "..")
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
        return Err(invalid());
    }
    let branch = fields.get("branch").cloned().unwrap_or_else(|| "main".into());
    if branch.len() > 255 || branch == "@" || branch.contains("..") || branch.contains("@{")
        || branch.chars().any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\#%".contains(c))
        || (!branch.is_empty() && branch.split('/').any(|s|
            s.is_empty() || s.starts_with('.') || s.ends_with('.') || s.ends_with(".lock"))) {
        return Err(invalid());
    }
    let enabled = match fields.get("enabled").map(String::as_str) {
        None | Some("true") => true,
        Some("false") => false,
        _ => return Err(invalid()),
    };
    Ok(Request { repo: SkillRepoEntry { owner: owner.into(), name: name.into(), branch, enabled }, directory, host })
}

#[cfg(test)]
#[path = "deeplink_tests.rs"]
mod tests;
