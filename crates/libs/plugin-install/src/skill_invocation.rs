//! Exact plugin mentions do not authorize their display names.
use regex::Regex;
use std::{collections::BTreeSet, sync::OnceLock};

/// 从原始用户文本提取精确 @ 插件链接，同时移除链接展示名的隐式授权影响。
/// $ 插件链接不是 @ 插件选择；查询参数仅用于宿主定向，不属于身份。
pub fn reference_text(text: &str) -> (String, BTreeSet<String>) {
    static LINK: OnceLock<Regex> = OnceLock::new();
    let link = LINK.get_or_init(|| {
        Regex::new(r"\[([@$])([A-Za-z0-9_:-]+)\]\s*\(([^)]*)\)")
            .expect("static plugin mention expression")
    });
    let mut ids = BTreeSet::new();
    let plain = link.replace_all(text, |captures: &regex::Captures<'_>| {
        let path = captures[3].trim();
        let Some(id) = path.strip_prefix("plugin://") else {
            return captures[0].to_string();
        };
        let id = id.split('?').next().unwrap_or_default();
        let common_env = matches!(
            captures[2].to_ascii_uppercase().as_str(),
            "PATH"
                | "HOME"
                | "USER"
                | "SHELL"
                | "PWD"
                | "TMPDIR"
                | "TEMP"
                | "TMP"
                | "LANG"
                | "TERM"
                | "XDG_CONFIG_HOME"
        );
        if &captures[1] == "@" && !id.is_empty() && !common_env {
            ids.insert(id.to_owned());
        }
        " ".to_owned()
    });
    (plain.into_owned(), ids)
}
