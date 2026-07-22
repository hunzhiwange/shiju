//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::server_config_unreachable_error;
use crate::app::{
    App, Message, load_query_classification_config, state::QueryClassificationRuleInput,
    update_query_classification_config_result,
};
use iced::Task;
use vw_config_types::routing::ClassificationRule;

use super::messages::{QueryClassificationMessage, SettingsMessage};

fn rules_for_config(app: &App) -> Vec<ClassificationRule> {
    app.query_classification_settings
        .rules
        .iter()
        .filter(|rule| {
            !rule.pattern.trim().is_empty()
                || !rule.category.trim().is_empty()
                || !rule.priority_input.trim().is_empty()
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

fn sync_to_config(app: &mut App) {
    let rules = rules_for_config(app);

    if let Err(err) = update_query_classification_config_result(|cfg| {
        cfg.enabled = app.query_classification_settings.enabled && !rules.is_empty();
        cfg.rules = rules;
    }) {
        app.query_classification_settings.save_error = Some(server_config_unreachable_error(err));
    }
}

fn rule_input_from_config(rule: ClassificationRule) -> QueryClassificationRuleInput {
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

fn validate(app: &App) -> Result<(), String> {
    for (idx, rule) in app.query_classification_settings.rules.iter().enumerate() {
        let pattern = rule.pattern.trim();
        let category = rule.category.trim();
        let priority = rule.priority_input.trim();

        if pattern.is_empty() && category.is_empty() && priority.is_empty() {
            continue;
        }
        if pattern.is_empty() {
            return Err(format!("第 {} 条分类规则缺少 pattern", idx + 1));
        }
        if category.is_empty() {
            return Err(format!("第 {} 条分类规则缺少 category", idx + 1));
        }
        if !priority.is_empty() && priority.parse::<i32>().is_err() {
            return Err(format!("第 {} 条分类规则的 priority 不是有效数字", idx + 1));
        }
    }
    Ok(())
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::QueryClassification(message) = message else {
        return Task::none();
    };

    match message {
        QueryClassificationMessage::Refresh => {
            let cfg = load_query_classification_config();
            app.query_classification_settings.enabled = cfg.enabled;
            app.query_classification_settings.rules =
                cfg.rules.into_iter().map(rule_input_from_config).collect();
            app.query_classification_settings.save_error = None;
            Task::none()
        }
        QueryClassificationMessage::EnabledToggled(value) => {
            app.query_classification_settings.enabled = value;
            app.query_classification_settings.save_error = None;
            sync_to_config(app);
            Task::none()
        }
        QueryClassificationMessage::AddRule => {
            app.query_classification_settings.rules.push(Default::default());
            app.query_classification_settings.save_error = None;
            sync_to_config(app);
            Task::none()
        }
        QueryClassificationMessage::RemoveRule(idx) => {
            if idx < app.query_classification_settings.rules.len() {
                app.query_classification_settings.rules.remove(idx);
            }
            app.query_classification_settings.save_error = None;
            sync_to_config(app);
            Task::none()
        }
        QueryClassificationMessage::PatternChanged(idx, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(idx) {
                rule.pattern = value;
            }
            match validate(app) {
                Ok(()) => {
                    app.query_classification_settings.save_error = None;
                    sync_to_config(app);
                }
                Err(err) => app.query_classification_settings.save_error = Some(err),
            }
            Task::none()
        }
        QueryClassificationMessage::CategoryChanged(idx, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(idx) {
                rule.category = value;
            }
            match validate(app) {
                Ok(()) => {
                    app.query_classification_settings.save_error = None;
                    sync_to_config(app);
                }
                Err(err) => app.query_classification_settings.save_error = Some(err),
            }
            Task::none()
        }
        QueryClassificationMessage::PriorityChanged(idx, value) => {
            if let Some(rule) = app.query_classification_settings.rules.get_mut(idx) {
                rule.priority_input = value;
            }
            match validate(app) {
                Ok(()) => {
                    app.query_classification_settings.save_error = None;
                    sync_to_config(app);
                }
                Err(err) => app.query_classification_settings.save_error = Some(err),
            }
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "query_classification_tests.rs"]
mod query_classification_tests;
