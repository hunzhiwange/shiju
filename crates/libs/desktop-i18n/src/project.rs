//! Project search feedback is rendered from semantic state with an explicit locale.
use super::{I18n, input::Feedback};

pub fn search_error(i18n: I18n, raw: &str, feedback: Option<&Feedback>) -> String {
    feedback.map(|message| message.render(i18n, raw)).unwrap_or_else(|| raw.to_owned())
}

/// Local filesystem diagnostics retain their template separately from OS/user values.
#[derive(Debug, Clone)]
pub struct ProjectFeedback {
    pub source: &'static str,
    pub arguments: Vec<super::format::Argument>,
    pub fallback: String,
}
impl ProjectFeedback {
    pub fn new(source: &'static str, arguments: Vec<super::format::Argument>) -> Self {
        let fallback = I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
            .format(source, &arguments)
            .expect("valid project feedback template");
        Self { source, arguments, fallback }
    }
}

pub fn worktree_label(i18n: I18n, directory: &str, name: &str, primary: bool) -> String {
    if directory == "__create_worktree__" {
        i18n.__("创建新的独立工作区").to_owned()
    } else if primary {
        i18n.__("主工作区").to_owned()
    } else {
        let name = if name.is_empty() { i18n.__("工作树") } else { name };
        i18n.format("独立工作区 (%s)", &[name.into()]).expect("valid worktree label template")
    }
}
