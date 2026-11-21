use std::fmt;

pub(super) type Directory = [(String, String)];

pub(super) fn name<'a>(directory: Option<&'a Directory>, path: &str) -> Option<&'a str> {
    directory?
        .iter()
        .find(|(key, _)| key.trim_matches('/') == path.trim_matches('/'))
        .map(|(_, name)| name.as_str())
}

pub(super) fn contains(prefix: &str, path: &str) -> bool {
    let path = path.trim_matches('/');
    let prefix = prefix.trim_matches('/');
    prefix.is_empty()
        || path == prefix
        || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with(['/', ',']))
}

#[derive(Clone, PartialEq)]
pub(super) struct Choice<T> {
    pub value: T,
    pub label: String,
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}
