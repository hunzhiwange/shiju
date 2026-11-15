use super::I18n;
pub use desktop_i18n::other::*;
pub(crate) fn tool_name(i18n: I18n, id: &str) -> String {
    let source = crate::app::state::tool_display_name(id);
    if source == id { source } else { i18n.__(&source).to_owned() }
}

pub(crate) fn session_title(app: &crate::app::App, id: &str, raw: &str) -> String {
    if raw.trim().is_empty() {
        return app
            .session_previews
            .get(id)
            .cloned()
            .unwrap_or_else(|| I18n::new(app.desktop_locale).__("新会话").to_owned());
    }
    app.generated_session_titles
        .get(id)
        .map(|feedback| feedback.render(I18n::new(app.desktop_locale), raw))
        .unwrap_or_else(|| raw.to_owned())
}
