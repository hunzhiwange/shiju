use super::I18n;
pub use desktop_i18n::apps::*;

/// Refresh only built-in hub titles; user-authored tab names remain unchanged.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn refresh_hub_titles(app: &mut crate::app::App) {
    use crate::app::message::{shiju_apps, skill_hub};
    let i18n = I18n::new(app.general_settings.desktop_locale);
    desktop_i18n::apps::refresh_hub_titles(
        i18n,
        app.navigation_tabs.open_tabs.iter_mut().map(|tab| (tab.id.as_str(), &mut tab.title)),
        &[
            (skill_hub::EXPERT_HUB_TAB_ID, skill_hub::EXPERT_HUB_TITLE),
            (skill_hub::SHIJU_SKILLS_TAB_ID, skill_hub::SHIJU_SKILLS_TITLE),
            (shiju_apps::SHIJU_APPS_TAB_ID, shiju_apps::SHIJU_APPS_TITLE),
        ],
    );
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn refresh_hub_titles(_app: &mut crate::app::App) {}
