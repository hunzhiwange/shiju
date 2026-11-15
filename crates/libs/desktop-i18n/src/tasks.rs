use super::{I18n, format::Message};

pub fn status_label(i18n: I18n, source: &str, custom: bool) -> String {
    if custom { source.to_owned() } else { i18n.__(source).to_owned() }
}

pub fn log_message(i18n: I18n, localized: Option<&Message>, transition: Option<(String, String)>, raw: &str) -> String {
    if let Some(message) = localized {
        return message.render(i18n).expect("valid task log template");
    }
    match transition {
        Some((from, to)) => i18n.format("状态变更: %s → %s", &[from.into(), to.into()]).expect("valid task status template"),
        None => raw.to_owned(),
    }
}
