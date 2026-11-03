//! Modal messages are formatted at display time using an explicit locale.
use super::{I18n, format::Argument};

pub(crate) fn render(i18n: I18n, source: &str, arguments: &[Argument]) -> String {
    super::format::Message::new(source, arguments.to_vec())
        .render(i18n)
        .expect("validated modal translation")
}
