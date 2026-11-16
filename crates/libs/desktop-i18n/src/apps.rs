//! App navigation search keeps source labels and stable identifiers searchable.
use super::I18n;

pub fn matches_navigation(i18n: I18n, source: &str, query: &str) -> bool {
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

/// Refresh known built-in labels using the identifiers supplied by their domains.
pub fn refresh_hub_titles<'a>(
    i18n: I18n,
    tabs: impl IntoIterator<Item = (&'a str, &'a mut String)>,
    sources: &[(&str, &str)],
) {
    for (id, title) in tabs {
        if let Some((_, source)) = sources.iter().find(|(key, _)| *key == id) {
            *title = i18n.__(source).to_owned();
        }
    }
}
