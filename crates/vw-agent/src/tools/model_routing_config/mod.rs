//! 模型路由配置工具
//!
//! 本模块提供了动态管理模型路由规则和委托代理配置的功能。
//! 支持添加、更新和删除路由规则，以及对场景和子智能体的完整生命周期管理。
//!
//! # 核心功能
//!
//! - **默认模型设置管理**：配置全局默认的提供方、模型和温度参数
//! - **场景路由管理**：基于 hint（提示符）的路由规则配置
//! - **分类规则管理**：关键词、模式、长度和优先级等分类器配置
//! - **委托代理管理**：子智能体的创建、更新和删除
//!
//! # 主要操作
//!
//! - `get`：获取当前完整的路由配置快照
//! - `list_hints`：列出所有可用的场景提示符
//! - `set_default`：设置默认提供方/模型/温度
//! - `upsert_scenario`：创建或更新场景路由及分类规则
//! - `remove_scenario`：删除场景路由及关联的分类规则
//! - `upsert_agent`：创建或更新委托子智能体
//! - `remove_agent`：删除委托子智能体

mod agents;
mod defaults;
mod parse;
mod scenarios;
mod snapshot;

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::{Config, get_project_without_env_blocking};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 委托智能体的默认最大递归深度
const DEFAULT_AGENT_MAX_DEPTH: u32 = 3;

/// 委托智能体的默认最大迭代次数（用于 agentic 模式）
const DEFAULT_AGENT_MAX_ITERATIONS: usize = 10;

/// 模型路由配置工具
///
/// 该工具实现了 Tool trait，提供完整的模型路由和委托代理配置管理能力。
/// 通过安全策略进行权限控制和速率限制。
pub struct ModelRoutingConfigTool {
    /// 应用配置的共享引用
    config: Arc<Config>,
    /// 安全策略的共享引用，用于权限检查
    security: Arc<SecurityPolicy>,
}

impl ModelRoutingConfigTool {
    /// 创建新的模型路由配置工具实例
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    /// 从数据库基础配置与项目覆盖加载配置（不包含环境变量覆盖）
    fn load_config_without_env(&self) -> anyhow::Result<Config> {
        get_project_without_env_blocking(&self.config.workspace_dir)
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    /// 检查写入权限和速率限制
    fn require_write_access(&self) -> Option<ToolResult> {
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        None
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ModelRoutingConfigTool {
    fn name(&self) -> &str {
        "model_routing_config"
    }

    fn description(&self) -> &str {
        "管理默认模型设置、基于场景的提供方/模型路由、分类规则和委托子智能体配置"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "get",
                        "list_hints",
                        "set_default",
                        "upsert_scenario",
                        "remove_scenario",
                        "upsert_agent",
                        "remove_agent"
                    ],
                    "default": "get"
                },
                "hint": {
                    "type": "string",
                    "description": "Scenario hint name (for example: conversation, coding, reasoning)"
                },
                "provider": {
                    "type": "string",
                    "description": "Provider for set_default/upsert_scenario/upsert_agent"
                },
                "model": {
                    "type": "string",
                    "description": "Model for set_default/upsert_scenario/upsert_agent"
                },
                "temperature": {
                    "type": ["number", "null"],
                    "description": "Optional temperature override (0.0-2.0)"
                },
                "api_key": {
                    "type": ["string", "null"],
                    "description": "Optional API key override for scenario route or delegate agent"
                },
                "keywords": {
                    "description": "Classification keywords for upsert_scenario (string or string array)",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "patterns": {
                    "description": "Classification literal patterns for upsert_scenario (string or string array)",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "min_length": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "description": "Optional minimum message length matcher"
                },
                "max_length": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "description": "Optional maximum message length matcher"
                },
                "priority": {
                    "type": ["integer", "null"],
                    "description": "Classification priority (higher runs first)"
                },
                "classification_enabled": {
                    "type": "boolean",
                    "description": "When true, upsert classification rule for this hint; false removes it"
                },
                "remove_classification": {
                    "type": "boolean",
                    "description": "When remove_scenario, whether to remove matching classification rule (default true)"
                },
                "name": {
                    "type": "string",
                    "description": "Delegate sub-agent name for upsert_agent/remove_agent"
                },
                "system_prompt": {
                    "type": ["string", "null"],
                    "description": "Optional system prompt override for delegate agent"
                },
                "max_depth": {
                    "type": ["integer", "null"],
                    "minimum": 1,
                    "description": "Delegate max recursion depth"
                },
                "agentic": {
                    "type": "boolean",
                    "description": "Enable tool-call loop mode for delegate agent"
                },
                "allowed_tools": {
                    "description": "Allowed tools for agentic delegate mode (string or string array)",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "allowed_skills": {
                    "description": "Allowed skills for delegate agent prompt context (string or string array)",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "max_iterations": {
                    "type": ["integer", "null"],
                    "minimum": 1,
                    "description": "Maximum tool-call iterations for agentic delegate mode"
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let action =
            args.get("action").and_then(Value::as_str).unwrap_or("get").to_ascii_lowercase();

        let result = match action.as_str() {
            "get" => self.handle_get(),
            "list_hints" => self.handle_list_hints(),
            "set_default" | "upsert_scenario" | "remove_scenario" | "upsert_agent"
            | "remove_agent" => {
                if let Some(blocked) = self.require_write_access() {
                    return Ok(blocked);
                }

                match action.as_str() {
                    "set_default" => self.handle_set_default(&args).await,
                    "upsert_scenario" => self.handle_upsert_scenario(&args).await,
                    "remove_scenario" => self.handle_remove_scenario(&args).await,
                    "upsert_agent" => self.handle_upsert_agent(&args).await,
                    "remove_agent" => self.handle_remove_agent(&args).await,
                    _ => unreachable!("validated above"),
                }
            }
            _ => anyhow::bail!(
                "Unknown action '{action}'. Valid: get, list_hints, set_default, upsert_scenario, remove_scenario, upsert_agent, remove_agent"
            ),
        };

        match result {
            Ok(outcome) => Ok(outcome),
            Err(error) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
            }),
        }
    }
}

/// 测试模块
///
/// 测试代码位于 tests/model_routing_config.rs
#[cfg(test)]
#[path = "../tests/model_routing_config.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
