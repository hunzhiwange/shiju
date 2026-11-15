//! Detect DSML wire markup before parsing or workflow response handling.
use regex::Regex;
use std::sync::LazyLock;

pub fn contains_deepseek_dsml_markup(response: &str) -> bool {
    static MARKER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)</?[｜|]+DSML[｜|]+").unwrap());
    MARKER_RE.is_match(response)
}
