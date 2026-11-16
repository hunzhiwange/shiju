use super::I18n;
pub use desktop_i18n::other::*;
pub(crate) fn tool_name(i18n: I18n, id: &str) -> String {
    let source = crate::app::state::tool_display_name(id);
    if source == id { source } else { i18n.__(&source).to_owned() }
}

pub(crate) fn session_title(app: &crate::app::App, id: &str, raw: &str) -> String {
    desktop_i18n::other::session_title(
        I18n::new(app.general_settings.desktop_locale),
        raw,
        app.session_state.session_previews.get(id).map(String::as_str),
        app.input_feedback.generated_session_titles.get(id),
    )
}
