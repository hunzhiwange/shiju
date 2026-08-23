//! Slash-command filtering helpers with no Codex command registry or dispatch.
use crate::vibe::action::V3SelectionItem;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ServiceTierCommand {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SlashCommandItem {
    Vibe(V3SelectionItem),
    ServiceTier(ServiceTierCommand),
}
impl SlashCommandItem {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Vibe(item) => &item.id,
            Self::ServiceTier(item) => &item.id,
        }
    }
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Vibe(item) => &item.label,
            Self::ServiceTier(item) => &item.name,
        }
    }
    pub(crate) fn description(&self) -> &str {
        match self {
            Self::Vibe(item) => item.description.as_deref().unwrap_or(""),
            Self::ServiceTier(item) => &item.description,
        }
    }
    pub(crate) fn available_during_task(&self) -> bool {
        true
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BuiltinCommandFlags;
pub(crate) fn filter_slash_commands(
    items: &[V3SelectionItem],
    query: &str,
) -> Vec<V3SelectionItem> {
    let query = query.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            query.is_empty()
                || item.label.to_lowercase().contains(&query)
                || item.id.to_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}
pub(crate) fn commands_from_vibe_data(items: Vec<V3SelectionItem>) -> Vec<SlashCommandItem> {
    items.into_iter().map(SlashCommandItem::Vibe).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slash_filter_uses_supplied_vibe_window_data() {
        let items = vec![
            V3SelectionItem::new("search", "Search"),
            V3SelectionItem::new("settings", "Settings"),
        ];
        assert_eq!(filter_slash_commands(&items, "set")[0].id, "settings");
    }
}
