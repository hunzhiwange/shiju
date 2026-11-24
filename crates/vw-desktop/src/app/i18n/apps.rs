use super::I18n;

/// Refresh only built-in hub titles; user-authored tab names remain unchanged.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn refresh_hub_titles(app: &mut crate::app::App) {
    crate::app::digital_employees::refresh_titles(app);
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

#[cfg(test)]
pub use desktop_i18n::apps::*;

/// Built-in shell labels are cached on tabs, independently of their current screen.
pub(crate) fn refresh_builtin_titles(app: &mut crate::app::App) {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    desktop_i18n::apps::refresh_hub_titles(
        i18n,
        app.navigation_tabs.open_tabs.iter_mut().map(|tab| (tab.id.as_str(), &mut tab.title)),
        &[
            ("cron", "定时任务"),
            ("projects", "项目管理"),
            (desktop_navigation::panel::tabs::EDITOR_TAB_ID, "编辑器"),
        ],
    );
    // Search labels identify tabs in display order, not the number of search matches.
    for tab in &mut app.navigation_tabs.open_tabs {
        if let Some(id) = tab.id.strip_prefix("find:") {
            if let Some(index) = app.project_find.find_results_tabs.iter().position(|result| result.id == id) {
                tab.title = i18n.format("查找结果 %u", &[(index + 1).into()])
                    .expect("valid search result title");
            }
        }
    }
}
