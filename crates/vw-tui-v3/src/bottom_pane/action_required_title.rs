//! Shared title text for overlays that require an explicit user action.

pub(crate) const ACTION_REQUIRED_PREVIEW_PREFIX: &str = "[ ! ] Action Required";

/// Product-neutral title parts.  A spinner is deliberately excluded so a waiting animation
/// cannot hide the fact that an approval or question is still pending.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActionRequiredTitleItem {
    Spinner,
    Approval,
    UserInput,
    Thread,
}

pub(crate) fn build_action_required_title_text<I, F>(
    prefix: &str,
    items: I,
    excluded_items: &[ActionRequiredTitleItem],
    mut value_for: F,
) -> String
where
    I: IntoIterator<Item = ActionRequiredTitleItem>,
    F: FnMut(ActionRequiredTitleItem) -> Option<String>,
{
    let mut parts = vec![prefix.to_string()];
    for item in items {
        if item == ActionRequiredTitleItem::Spinner || excluded_items.contains(&item) {
            continue;
        }
        if let Some(value) = value_for(item) {
            parts.push(value);
        }
    }
    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_required_title_keeps_security_surface_visible() {
        assert_eq!(
            build_action_required_title_text(
                ACTION_REQUIRED_PREVIEW_PREFIX,
                [ActionRequiredTitleItem::Spinner, ActionRequiredTitleItem::Approval],
                &[],
                |item| (item == ActionRequiredTitleItem::Approval).then(|| "approval".to_string()),
            ),
            "[ ! ] Action Required | approval"
        );
    }
}
