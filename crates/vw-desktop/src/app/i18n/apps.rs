//! App navigation search keeps source labels and stable identifiers searchable.
use super::I18n;

pub(crate) fn matches_navigation(i18n: I18n, source: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    let identifier = match source {
        "项目管理" => "projects",
        "添加网址" => "add_url",
        "知识库" => "knowledge",
        "工作流" => "workflow",
        "任务编排" => "workflow_manager",
        _ => source,
    };
    query.is_empty()
        || [source, i18n.__(source), identifier]
            .iter()
            .any(|value| value.to_lowercase().contains(&query))
}

/// Refresh only built-in hub titles; user-authored tab names remain unchanged.
pub(crate) fn refresh_hub_titles(app: &mut crate::app::App) {
    use crate::app::message::{shiju_apps, skill_hub};
    let i18n = I18n::new(app.desktop_locale);
    for tab in &mut app.open_tabs {
        let source = match tab.id.as_str() {
            skill_hub::EXPERT_HUB_TAB_ID => skill_hub::EXPERT_HUB_TITLE,
            skill_hub::SHIJU_SKILLS_TAB_ID => skill_hub::SHIJU_SKILLS_TITLE,
            shiju_apps::SHIJU_APPS_TAB_ID => shiju_apps::SHIJU_APPS_TITLE,
            _ => continue,
        };
        tab.title = i18n.__(source).to_string();
    }
}
