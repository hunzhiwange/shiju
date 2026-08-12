use super::ModelRoutingConfigTool;
use crate::app::agent::config::{ClassificationRule, Config, ModelRouteConfig};
use serde_json::{Value, json};
use std::collections::BTreeMap;

impl ModelRoutingConfigTool {
    /// 构建场景行的 JSON 表示。
    fn scenario_row(route: &ModelRouteConfig, rule: Option<&ClassificationRule>) -> Value {
        let classification = rule.map(|r| {
            json!({
                "keywords": r.keywords,
                "patterns": r.patterns,
                "min_length": r.min_length,
                "max_length": r.max_length,
                "priority": r.priority,
            })
        });

        json!({
            "hint": route.hint,
            "provider": route.provider,
            "model": route.model,
            "api_key_configured": route
                .api_key
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()),
            "classification": classification,
        })
    }

    /// 生成配置快照。
    pub(super) fn snapshot(cfg: &Config) -> Value {
        let mut routes = cfg.model_routes.clone();
        routes.sort_by(|a, b| a.hint.cmp(&b.hint));

        let mut rules = cfg.query_classification.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.hint.cmp(&b.hint)));

        let mut scenarios = Vec::with_capacity(routes.len());
        for route in &routes {
            let rule = rules.iter().find(|r| r.hint == route.hint);
            scenarios.push(Self::scenario_row(route, rule));
        }

        let classification_only_rules: Vec<Value> = rules
            .iter()
            .filter(|rule| !routes.iter().any(|route| route.hint == rule.hint))
            .map(|rule| {
                json!({
                    "hint": rule.hint,
                    "keywords": rule.keywords,
                    "patterns": rule.patterns,
                    "min_length": rule.min_length,
                    "max_length": rule.max_length,
                    "priority": rule.priority,
                })
            })
            .collect();

        let mut agents: BTreeMap<String, Value> = BTreeMap::new();
        for (name, agent) in &cfg.agents {
            agents.insert(
                name.clone(),
                json!({
                    "provider": agent.provider,
                    "model": agent.model,
                    "system_prompt": agent.system_prompt,
                    "api_key_configured": agent
                        .api_key
                        .as_ref()
                        .is_some_and(|value| !value.trim().is_empty()),
                    "temperature": agent.temperature,
                    "max_depth": agent.max_depth,
                    "agentic": agent.agentic,
                    "tool_policy": agent.tool_policy,
                    "allowed_tools": agent.allowed_tools,
                    "skill_policy": agent.skill_policy,
                    "allowed_skills": agent.allowed_skills,
                    "max_iterations": agent.max_iterations,
                }),
            );
        }

        json!({
            "default": {
                "provider": cfg.default_provider,
                "model": cfg.default_model,
                "temperature": cfg.default_temperature,
            },
            "query_classification": {
                "enabled": cfg.query_classification.enabled,
                "rules_count": cfg.query_classification.rules.len(),
            },
            "scenarios": scenarios,
            "classification_only_rules": classification_only_rules,
            "agents": agents,
        })
    }

    /// 规范化并排序模型路由列表。
    pub(super) fn normalize_and_sort_routes(routes: &mut Vec<ModelRouteConfig>) {
        routes.retain(|route| !route.hint.trim().is_empty());
        routes.sort_by(|a, b| a.hint.cmp(&b.hint));
    }

    /// 规范化并排序分类规则列表。
    pub(super) fn normalize_and_sort_rules(rules: &mut Vec<ClassificationRule>) {
        rules.retain(|rule| !rule.hint.trim().is_empty());
        rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.hint.cmp(&b.hint)));
    }

    /// 检查规则是否包含任何匹配器。
    pub(super) fn has_rule_matcher(rule: &ClassificationRule) -> bool {
        !rule.keywords.is_empty()
            || !rule.patterns.is_empty()
            || rule.min_length.is_some()
            || rule.max_length.is_some()
    }

    /// 确保规则具有默认匹配器。
    pub(super) fn ensure_rule_defaults(rule: &mut ClassificationRule, hint: &str) {
        if !Self::has_rule_matcher(rule) {
            rule.keywords = vec![hint.to_string()];
        }
    }
}
#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
