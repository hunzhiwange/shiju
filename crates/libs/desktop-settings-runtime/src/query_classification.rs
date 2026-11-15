use crate::state::*;
use crate::messages::*;
use config_routing::ClassificationRule;
pub enum Effect { None, Refresh, Save(serde_json::Value), Invalid }
fn rules_for_config(state: &QueryClassificationSettingsState) -> Vec<ClassificationRule> {
    state
        .rules
        .iter()
        .filter(|rule| {
            !rule.pattern.trim().is_empty()
                || !rule.category.trim().is_empty()
                || (!rule.priority_input.trim().is_empty() && rule.priority_input.trim() != "0")
        })
        .map(|rule| ClassificationRule {
            hint: rule.category.trim().to_string(),
            keywords: if rule.pattern.trim().is_empty() {
                Vec::new()
            } else {
                vec![rule.pattern.trim().to_string()]
            },
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: rule.priority_input.trim().parse::<i32>().unwrap_or(0),
        })
        .collect()
}

pub fn rule_input_from_config(rule: ClassificationRule) -> QueryClassificationRuleInput {
    QueryClassificationRuleInput {
        pattern: rule
            .keywords
            .into_iter()
            .next()
            .or_else(|| rule.patterns.into_iter().next())
            .unwrap_or_default(),
        category: rule.hint,
        priority_input: rule.priority.to_string(),
    }
}

fn validation_message(state: &QueryClassificationSettingsState) -> Result<(), desktop_i18n::format::Message> {
    for (idx, rule) in state.rules.iter().enumerate() {
        let pattern = rule.pattern.trim();
        let category = rule.category.trim();
        let priority = rule.priority_input.trim();

        if pattern.is_empty() && category.is_empty() && (priority.is_empty() || priority == "0") {
            continue;
        }
        if pattern.is_empty() {
            return Err(desktop_i18n::format::Message::new(
                "第 %u 条分类规则缺少 pattern",
                vec![(idx + 1).into()],
            ));
        }
        if category.is_empty() {
            return Err(desktop_i18n::format::Message::new(
                "第 %u 条分类规则缺少 category",
                vec![(idx + 1).into()],
            ));
        }
        if !priority.is_empty() && priority.parse::<i32>().is_err() {
            return Err(desktop_i18n::format::Message::new(
                "第 %u 条分类规则的 priority 不是有效数字",
                vec![(idx + 1).into()],
            ));
        }
    }
    Ok(())
}

pub fn update(state: &mut QueryClassificationSettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::QueryClassification(message) = message else {
        return Effect::None;
    };
    match message {
        QueryClassificationMessage::Refresh => return Effect::Refresh,
        QueryClassificationMessage::EnabledToggled(value) => {
            state.enabled = value
        }
        QueryClassificationMessage::AddRule => {
            state.rules.push(Default::default())
        }
        QueryClassificationMessage::RemoveRule(index) => {
            if index < state.rules.len() {
                state.rules.remove(index);
            }
        }
        QueryClassificationMessage::PatternChanged(index, value) => {
            if let Some(rule) = state.rules.get_mut(index) {
                rule.pattern = value;
            }
        }
        QueryClassificationMessage::CategoryChanged(index, value) => {
            if let Some(rule) = state.rules.get_mut(index) {
                rule.category = value;
            }
        }
        QueryClassificationMessage::PriorityChanged(index, value) => {
            if let Some(rule) = state.rules.get_mut(index) {
                rule.priority_input = value;
            }
        }
    }

    match validation_message(state) {
        Ok(()) => {

            state.save_error = None;
            state.save_error_translation = None;
            let rules = rules_for_config(state);
            Effect::Save(
                serde_json::json!({"query_classification": {
                    "enabled": state.enabled && !rules.is_empty(), "rules": rules
                }}),
            )
        }
        Err(error) => {

            state.save_error = Some(
                error
                    .render(desktop_i18n::I18n::new(
                        config_ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .expect("valid classification error template"),
            );
            state.save_error_translation = Some(error);
            Effect::Invalid
        }
    }
}
#[cfg(test)]
#[path = "query_classification_tests.rs"]
mod query_classification_tests;
