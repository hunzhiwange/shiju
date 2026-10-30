//! 插件限定调用使用已审计身份；展示名与 Markdown 标签不授予其他来源权限。

use crate::skills::Skill;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

impl Skill {
    /// 市场身份属于调用键；原始 frontmatter 名称及文件字节保持不变。
    pub(crate) fn invocation_name(&self) -> String {
        match &self.plugin_source {
            Some(source) => format!("{}:{}", source.plugin_id, self.name),
            None => self.name.clone(),
        }
    }

    pub(crate) fn invocation_aliases(&self) -> Vec<String> {
        let mut names = vec![self.invocation_name()];
        let alias = match &self.plugin_source {
            Some(source) => Some(format!("{}:{}", source.namespace, self.name)),
            None => self.file_id(),
        };
        if let Some(alias) = alias
            && !names.iter().any(|name| name.eq_ignore_ascii_case(&alias))
        {
            names.push(alias);
        }
        names
    }

    /// 只有本地技能保留裸文件别名，插件文件名不能跨来源匹配。
    pub(crate) fn file_id(&self) -> Option<String> {
        if self.plugin_source.is_some() {
            return None;
        }
        let location = self.location.as_ref()?;
        let name = location.file_name()?.to_str()?;
        if name.eq_ignore_ascii_case("SKILL.md") || name.eq_ignore_ascii_case("SKILL.toml") {
            location.parent()?.file_name()?.to_str().map(ToOwned::to_owned)
        } else {
            location.file_stem()?.to_str().map(ToOwned::to_owned)
        }
    }
}

/// 从原始用户文本提取精确 @ 插件链接，同时移除链接展示名的隐式授权影响。
/// $ 插件链接不是 @ 插件选择；查询参数仅用于宿主定向，不属于身份。
pub(crate) fn reference_text(text: &str) -> (String, BTreeSet<String>) {
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
