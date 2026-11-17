use super::I18n;
pub(crate) use desktop_chat_tools::state::localized_tool_name as tool_name;

pub(crate) fn session_title(app: &crate::app::App, id: &str, raw: &str) -> String {
    desktop_i18n::other::session_title(
        I18n::new(app.general_settings.desktop_locale),
        raw,
        app.session_state.session_previews.get(id).map(String::as_str),
        app.input_feedback.generated_session_titles.get(id),
    )
}

#[cfg(test)]
pub use desktop_i18n::other::*;
