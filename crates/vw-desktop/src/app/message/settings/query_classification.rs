//! 查询分类草稿校验与共享异步保存。
use super::messages::{QueryClassificationMessage, SettingsMessage};
use super::routing_persistence;
use crate::app::{App, Message, state::QueryClassificationRuleInput};
use iced::Task;
use vw_config_types::routing::ClassificationRule;

fn rules_for_config(app: &App) -> Vec<ClassificationRule> {
    app.query_classification_settings
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

pub(super) fn rule_input_from_config(rule: ClassificationRule) -> QueryClassificationRuleInput {
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

fn validation_message(app: &App) -> Result<(), crate::app::i18n::format::Message> {
    for (idx, rule) in app.query_classification_settings.rules.iter().enumerate() {
        let pattern = rule.pattern.trim();
        let category = rule.category.trim();
        let priority = rule.priority_input.trim();

        if pattern.is_empty() && category.is_empty() && (priority.is_empty() || priority == "0") {
            continue;
        }
        if pattern.is_empty() {
            return Err(crate::app::i18n::format::Message::new(
                "第 %u 条分类规则缺少 pattern",
                vec![(idx + 1).into()],
            ));
        }
        if category.is_empty() {
            return Err(crate::app::i18n::format::Message::new(
                "第 %u 条分类规则缺少 category",
                vec![(idx + 1).into()],
            ));
        }
        if !priority.is_empty() && priority.parse::<i32>().is_err() {
            return Err(crate::app::i18n::format::Message::new(
                "第 %u 条分类规则的 priority 不是有效数字",
                vec![(idx + 1).into()],
            ));
        }
    }
    Ok(())
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::QueryClassification(message) = message else {
        return Task::none();
    };
    match message {
        QueryClassificationMessage::Refresh => return routing_persistence::refresh(app),
        QueryClassificationMessage::EnabledToggled(value) => {
            app.query_classification_settings.enabled = value
        }
        QueryClassificationMessage::AddRule => {
            app.query_classification_settings.rules.push(Default::default())
        }
        QueryClassificationMessage::RemoveRule(index) => {
            if index < app.query_classification_settings.rules.len() {
                app.query_classification_settings.rules.remove(index);
            }
        }
        QueryClassificationMessage::PatternChanged(index, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(index) {
                rule.pattern = value;
            }
        }
        QueryClassificationMessage::CategoryChanged(index, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(index) {
                rule.category = value;
            }
        }
        QueryClassificationMessage::PriorityChanged(index, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(index) {
                rule.priority_input = value;
            }
        }
    }
    app.model_routes_settings.persistence.edited();
    match validation_message(app) {
        Ok(()) => {
            app.model_routes_settings.persistence.classification_invalid = false;
            app.query_classification_settings.save_error = None;
            app.query_classification_settings.save_error_translation = None;
            let rules = rules_for_config(app);
            routing_persistence::enqueue(
                app,
                serde_json::json!({"query_classification": {
                    "enabled": app.query_classification_settings.enabled && !rules.is_empty(), "rules": rules
                }}),
            )
        }
        Err(error) => {
            app.model_routes_settings.persistence.classification_invalid = true;
            app.query_classification_settings.save_error = Some(
                error
                    .render(crate::app::i18n::I18n::new(
                        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .expect("valid classification error template"),
            );
            app.query_classification_settings.save_error_translation = Some(error);
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "query_classification_tests.rs"]
mod query_classification_tests;

#[cfg(test)]
fn validate(app: &App) -> Result<(), String> {
    validation_message(app).map_err(|message| {
        message
            .render(crate::app::i18n::I18n::new(
                vw_config_types::ui::DesktopLocale::SimplifiedChinese,
            ))
            .expect("valid classification error template")
    })
}
