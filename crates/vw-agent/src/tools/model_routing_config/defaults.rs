use super::ModelRoutingConfigTool;
use crate::app::agent::config::{self, Config};
use crate::app::agent::util::MaybeSet;
use serde_json::{Value, json};

use super::super::traits::ToolResult;

impl ModelRoutingConfigTool {
    /// 处理获取配置请求。
    pub(super) fn handle_get(&self) -> anyhow::Result<ToolResult> {
        let cfg = self.load_config_without_env()?;
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&Self::snapshot(&cfg))?,
            error: None,
        })
    }

    /// 处理列出提示符请求。
    pub(super) fn handle_list_hints(&self) -> anyhow::Result<ToolResult> {
        let cfg = self.load_config_without_env()?;

        let mut route_hints: Vec<String> =
            cfg.model_routes.iter().map(|r| r.hint.clone()).collect();
        route_hints.sort();
        route_hints.dedup();

        let mut classification_hints: Vec<String> =
            cfg.query_classification.rules.iter().map(|r| r.hint.clone()).collect();
        classification_hints.sort();
        classification_hints.dedup();

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "model_route_hints": route_hints,
                "classification_hints": classification_hints,
                "example": {
                    "conversation": {
                        "action": "upsert_scenario",
                        "hint": "conversation",
                        "provider": "kimi",
                        "model": "moonshot-v1-8k",
                        "classification_enabled": false
                    },
                    "coding": {
                        "action": "upsert_scenario",
                        "hint": "coding",
                        "provider": "openai",
                        "model": "gpt-5.3-codex",
                        "classification_enabled": true,
                        "keywords": ["code", "bug", "refactor", "test"],
                        "patterns": ["```"],
                        "priority": 50
                    }
                }
            }))?,
            error: None,
        })
    }

    /// 处理设置默认配置请求。
    pub(super) async fn handle_set_default(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let provider_update = Self::parse_optional_string_update(args, "provider")?;
        let model_update = Self::parse_optional_string_update(args, "model")?;
        let temperature_update = Self::parse_optional_f64_update(args, "temperature")?;

        let any_update = !matches!(provider_update, MaybeSet::Unset)
            || !matches!(model_update, MaybeSet::Unset)
            || !matches!(temperature_update, MaybeSet::Unset);

        if !any_update {
            anyhow::bail!("set_default requires at least one of: provider, model, temperature");
        }

        let mut cfg = config::get_project_blocking(&self.config.workspace_dir)?;
        let before = cfg.clone();

        match provider_update {
            MaybeSet::Set(provider) => cfg.default_provider = Some(provider),
            MaybeSet::Null => cfg.default_provider = None,
            MaybeSet::Unset => {}
        }

        match model_update {
            MaybeSet::Set(model) => cfg.default_model = Some(model),
            MaybeSet::Null => cfg.default_model = None,
            MaybeSet::Unset => {}
        }

        match temperature_update {
            MaybeSet::Set(temperature) => {
                if !(0.0..=2.0).contains(&temperature) {
                    anyhow::bail!("'temperature' must be between 0.0 and 2.0");
                }
                cfg.default_temperature = temperature;
            }
            MaybeSet::Null => {
                cfg.default_temperature = Config::default().default_temperature;
            }
            MaybeSet::Unset => {}
        }

        cfg =
            config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Default provider/model settings updated",
                "config": Self::snapshot(&cfg),
            }))?,
            error: None,
        })
    }
}
#[cfg(test)]
#[path = "defaults_tests.rs"]
mod defaults_tests;
