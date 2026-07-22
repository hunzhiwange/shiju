use super::ModelRoutingConfigTool;
use crate::app::agent::config::{self, ClassificationRule, ModelRouteConfig};
use crate::app::agent::util::MaybeSet;
use serde_json::{Value, json};

use super::super::traits::ToolResult;

impl ModelRoutingConfigTool {
    /// 处理创建或更新场景请求。
    pub(super) async fn handle_upsert_scenario(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let hint = Self::parse_non_empty_string(args, "hint")?;
        let provider = Self::parse_non_empty_string(args, "provider")?;
        let model = Self::parse_non_empty_string(args, "model")?;
        let api_key_update = Self::parse_optional_string_update(args, "api_key")?;

        let keywords_update = if let Some(raw) = args.get("keywords") {
            Some(Self::parse_string_list(raw, "keywords")?)
        } else {
            None
        };
        let patterns_update = if let Some(raw) = args.get("patterns") {
            Some(Self::parse_string_list(raw, "patterns")?)
        } else {
            None
        };
        let min_length_update = Self::parse_optional_usize_update(args, "min_length")?;
        let max_length_update = Self::parse_optional_usize_update(args, "max_length")?;
        let priority_update = Self::parse_optional_i32_update(args, "priority")?;
        let classification_enabled = Self::parse_optional_bool(args, "classification_enabled")?;

        let should_touch_rule = classification_enabled.is_some()
            || keywords_update.is_some()
            || patterns_update.is_some()
            || !matches!(min_length_update, MaybeSet::Unset)
            || !matches!(max_length_update, MaybeSet::Unset)
            || !matches!(priority_update, MaybeSet::Unset);

        let mut cfg = config::get_project_blocking(&self.config.workspace_dir)?;
        let before = cfg.clone();

        let existing_route = cfg.model_routes.iter().find(|route| route.hint == hint).cloned();
        let mut next_route = existing_route.unwrap_or(ModelRouteConfig {
            hint: hint.clone(),
            provider: provider.clone(),
            model: model.clone(),
            max_tokens: None,
            api_key: None,
        });

        next_route.hint = hint.clone();
        next_route.provider = provider;
        next_route.model = model;

        match api_key_update {
            MaybeSet::Set(api_key) => next_route.api_key = Some(api_key),
            MaybeSet::Null => next_route.api_key = None,
            MaybeSet::Unset => {}
        }

        cfg.model_routes.retain(|route| route.hint != hint);
        cfg.model_routes.push(next_route);
        Self::normalize_and_sort_routes(&mut cfg.model_routes);

        if should_touch_rule {
            if matches!(classification_enabled, Some(false)) {
                cfg.query_classification.rules.retain(|rule| rule.hint != hint);
            } else {
                let existing_rule =
                    cfg.query_classification.rules.iter().find(|rule| rule.hint == hint).cloned();

                let mut next_rule = existing_rule.unwrap_or_else(|| ClassificationRule {
                    hint: hint.clone(),
                    ..ClassificationRule::default()
                });

                if let Some(keywords) = keywords_update {
                    next_rule.keywords = keywords;
                }
                if let Some(patterns) = patterns_update {
                    next_rule.patterns = patterns;
                }

                match min_length_update {
                    MaybeSet::Set(value) => next_rule.min_length = Some(value),
                    MaybeSet::Null => next_rule.min_length = None,
                    MaybeSet::Unset => {}
                }

                match max_length_update {
                    MaybeSet::Set(value) => next_rule.max_length = Some(value),
                    MaybeSet::Null => next_rule.max_length = None,
                    MaybeSet::Unset => {}
                }

                match priority_update {
                    MaybeSet::Set(value) => next_rule.priority = value,
                    MaybeSet::Null => next_rule.priority = 0,
                    MaybeSet::Unset => {}
                }

                if matches!(classification_enabled, Some(true)) {
                    Self::ensure_rule_defaults(&mut next_rule, &hint);
                }

                if !Self::has_rule_matcher(&next_rule) {
                    anyhow::bail!(
                        "Classification rule for hint '{hint}' has no matching criteria. Provide keywords/patterns or set min_length/max_length."
                    );
                }

                cfg.query_classification.rules.retain(|rule| rule.hint != hint);
                cfg.query_classification.rules.push(next_rule);
            }
        }

        Self::normalize_and_sort_rules(&mut cfg.query_classification.rules);
        cfg.query_classification.enabled = !cfg.query_classification.rules.is_empty();

        cfg =
            config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Scenario route upserted",
                "hint": hint,
                "config": Self::snapshot(&cfg),
            }))?,
            error: None,
        })
    }

    /// 处理删除场景请求。
    pub(super) async fn handle_remove_scenario(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let hint = Self::parse_non_empty_string(args, "hint")?;
        let remove_classification =
            args.get("remove_classification").and_then(Value::as_bool).unwrap_or(true);

        let mut cfg = config::get_project_blocking(&self.config.workspace_dir)?;
        let before = cfg.clone();

        let before_routes = cfg.model_routes.len();
        cfg.model_routes.retain(|route| route.hint != hint);
        let routes_removed = before_routes.saturating_sub(cfg.model_routes.len());

        let mut rules_removed = 0usize;
        if remove_classification {
            let before_rules = cfg.query_classification.rules.len();
            cfg.query_classification.rules.retain(|rule| rule.hint != hint);
            rules_removed = before_rules.saturating_sub(cfg.query_classification.rules.len());
        }

        if routes_removed == 0 && rules_removed == 0 {
            anyhow::bail!("No scenario found for hint '{hint}'");
        }

        Self::normalize_and_sort_routes(&mut cfg.model_routes);
        Self::normalize_and_sort_rules(&mut cfg.query_classification.rules);
        cfg.query_classification.enabled = !cfg.query_classification.rules.is_empty();

        cfg =
            config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Scenario removed",
                "hint": hint,
                "routes_removed": routes_removed,
                "classification_rules_removed": rules_removed,
                "config": Self::snapshot(&cfg),
            }))?,
            error: None,
        })
    }
}
#[cfg(test)]
#[path = "scenarios_tests.rs"]
mod scenarios_tests;
