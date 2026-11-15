//! 模型标识、搜索及展示标签。
use desktop_settings_providers::state::ProviderModelsSummary;

pub(crate) fn normalize_model_input(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() }
}

pub(crate) fn model_matches_query(
    query: &str,
    provider_id: &str,
    provider_name: &str,
    model_id: &str,
    model_name: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    provider_id.to_ascii_lowercase().contains(query)
        || provider_name.to_ascii_lowercase().contains(query)
        || model_id.to_ascii_lowercase().contains(query)
        || model_name.to_ascii_lowercase().contains(query)
}

pub(crate) fn model_is_selected(current: &str, provider_id: &str, model_id: &str) -> bool {
    if current.contains('/') {
        current == format!("{}/{}", provider_id, model_id)
    } else {
        current == model_id
    }
}

pub(crate) fn truncated_model_name(name: &str) -> String {
    let max_chars = 26usize;
    if name.chars().count() > max_chars {
        name.chars().take(max_chars.saturating_sub(1)).collect::<String>() + "…"
    } else {
        name.to_string()
    }
}

pub fn model_toggle_label(
    providers: &[ProviderModelsSummary],
    auto_model: bool,
    model: &str,
) -> String {
    if auto_model {
        return "自动模型".to_string();
    }

    let parsed = model.split_once('/');
    let display = match parsed {
        Some((provider_id, model_id)) => providers
            .iter()
            .find(|p| p.id.as_str() == provider_id)
            .and_then(|p| p.models.iter().find(|m| m.id.as_str() == model_id))
            .map(|m| m.name.clone()),
        None => providers
            .iter()
            .find_map(|p| p.models.iter().find(|m| m.id.as_str() == model).map(|m| m.name.clone())),
    };

    display.unwrap_or_else(|| match parsed {
        Some((_, model_id)) => model_id.to_string(),
        None => model.to_string(),
    })
}

pub(crate) fn model_provider_icon_id(providers: &[ProviderModelsSummary], model: &str) -> String {
    if model.contains('/') {
        model.split('/').next().unwrap_or("agent").to_string()
    } else {
        providers
            .iter()
            .find_map(|p| p.models.iter().any(|m| m.id.as_str() == model).then_some(p.id.clone()))
            .unwrap_or_else(|| "agent".to_string())
    }
}

#[cfg(test)]
#[path = "model_popover_tests.rs"]
mod tests;
