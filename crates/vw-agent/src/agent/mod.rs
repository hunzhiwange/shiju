//! # Agent 模块
//!
//! 本模块是 VibeWindow 代理系统的核心编排层，负责代理的定义、加载和管理。
//!
//! ## 主要功能
//!
//! - **代理信息管理**：定义和存储各种代理的配置信息
//! - **状态加载**：从配置文件加载代理状态，包括内置代理和用户自定义代理
//! - **权限控制**：为每个代理配置细粒度的权限规则
//! - **默认代理选择**：智能选择合适的默认代理
//!
//! ## 模块结构
//!
//! - `agent`：核心代理实现，包含 Agent 结构体和 AgentBuilder
//! - `classifier`：消息分类器，用于确定消息类型
//! - `dispatcher`：任务调度器，负责任务分发
//! - `loop_`：主循环处理，包含消息处理和运行逻辑
//! - `memory_loader`：记忆加载器
//! - `prompt`：提示词管理
//!
//! ## 内置代理
//!
//! 本模块预定义了以下内置代理：
//! - `main`：唯一交互执行代理
//! - `researcher`、`coder`、`reviewer`、`explore`：专业委托代理

#[allow(clippy::module_inception)]
pub mod agent;
pub mod classifier;
pub mod dispatcher;
pub mod loop_;
pub mod memory_loader;
pub mod prompt;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use agent::run;
#[allow(unused_imports)]
pub use agent::{Agent, AgentBuilder};
#[allow(unused_imports)]
pub use loop_::process_message;

use crate::app::agent::global;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::permission::next::Ruleset;
use crate::app::agent::provider::provider;
use crate::app::agent::skill;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use vw_config_types::agent::{DelegateAgentConfig, merged_agent_configs};

/// 代理生成提示词模板
///
/// 用于生成代理配置的提示词内容
const PROMPT_GENERATE: &str = include_str!("generate.txt");

/// 探索提示词模板
///
/// 用于 explore 代理执行代码库探索任务
const PROMPT_EXPLORE: &str = include_str!("prompt/explore.txt");

/// 研究角色提示词模板。
const PROMPT_RESEARCHER: &str = include_str!("prompt/researcher.txt");

/// 实现角色提示词模板。
const PROMPT_CODER: &str = include_str!("prompt/coder.txt");

/// 审查角色提示词模板。
const PROMPT_REVIEWER: &str = include_str!("prompt/reviewer.txt");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub key: String,
    #[serde(flatten)]
    pub definition: DelegateAgentConfig,
}

#[derive(Debug, Clone)]
struct ResolvedAgentDefinition {
    definition: DelegateAgentConfig,
    permission: Ruleset,
}

/// 根据名称获取代理信息
///
/// 从当前租户与项目的最新配置中查找指定名称的代理。
///
/// # 参数
///
/// * `name` - 代理名称
///
/// # 返回值
///
/// 如果找到匹配的代理，返回 `Some(DelegateAgentConfig)`；否则返回 `None`。
///
/// # 示例
///
/// ```ignore
/// if let Some(info) = get("main").await {
///     println!("代理描述: {:?}", info.description);
/// }
/// ```
pub async fn get(name: &str) -> Option<DelegateAgentConfig> {
    load_state().await.ok()?.agents.get(name).map(|agent| agent.definition.clone())
}

/// 获取默认代理名称
///
/// 根据配置和可用代理，智能选择最合适的默认代理。
/// 选择优先级：
/// 1. 配置中明确指定的默认代理
/// 2. "main" 代理（如果可见且为主代理）
/// 3. 按名称排序的第一个可见主代理
///
/// # 返回值
///
/// 成功时返回默认代理名称 `Ok(String)`；
/// 失败时返回错误信息 `Err(String)`，可能的原因包括：
/// - 指定的默认代理不存在
/// - 指定的默认代理是子代理
/// - 指定的默认代理被隐藏
/// - 没有可用的可见主代理
///
/// # 示例
///
/// ```ignore
/// match default_agent().await {
///     Ok(name) => println!("默认代理: {}", name),
///     Err(e) => eprintln!("无法确定默认代理: {}", e),
/// }
/// ```
pub async fn default_agent() -> Result<String, String> {
    let state = load_state().await?;
    choose_default_agent(&state)
}

/// 获取所有代理列表
///
/// 返回所有已加载代理的信息列表，按优先级排序：
/// - 默认代理排在最前面
/// - 其余代理按名称字母顺序排列
///
/// # 返回值
///
/// 返回包含所有代理定义的列表视图
///
/// # 示例
///
/// ```ignore
/// let agents = list().await;
/// for agent in agents {
///     println!("{}: {:?}", agent.name, agent.description);
/// }
/// ```
pub async fn list() -> Vec<Info> {
    let state = match load_state().await {
        Ok(state) => state,
        Err(error) => {
            tracing::error!(%error, "failed to load agent definitions");
            return Vec::new();
        }
    };
    let mut out = state
        .agents
        .iter()
        .map(|(key, agent)| Info { key: key.clone(), definition: agent.definition.clone() })
        .collect::<Vec<_>>();

    // 获取首选代理名称，默认为 "main"
    let preferred = state.default_agent.as_deref().unwrap_or("main").to_string();

    // 排序：首选代理优先，其余按名称字母顺序
    out.sort_by(|a, b| {
        let a0 = a.key == preferred;
        let b0 = b.key == preferred;
        b0.cmp(&a0).then_with(|| a.key.cmp(&b.key))
    });
    out
}

/// 代理状态结构体
///
/// 存储所有代理的运行时状态，包括代理映射和默认代理名称。
#[derive(Debug, Default)]
struct State {
    /// 代理名称到代理信息的映射
    agents: HashMap<String, ResolvedAgentDefinition>,

    /// 默认代理名称
    default_agent: Option<String>,
}

/// 保留启动入口；代理定义在请求时读取，避免缓存覆盖已保存的配置。
pub fn init() {}

/// 判断代理是否为主要且可见的
///
/// 检查代理是否可以作为默认代理候选：
/// - 不是子代理（mode != "subagent"）
/// - 未被隐藏（hidden != true）
///
/// # 参数
///
/// * `agent` - 要检查的代理信息
///
/// # 返回值
///
/// 如果代理是可见的主代理，返回 `true`；否则返回 `false`
fn is_primary_visible(agent: &DelegateAgentConfig) -> bool {
    agent.mode != "subagent" && !agent.hidden
}

/// 从状态中选择默认代理
///
/// 内部函数，实现默认代理的选择逻辑。
///
/// # 参数
///
/// * `state` - 代理状态引用
///
/// # 返回值
///
/// 成功时返回默认代理名称；失败时返回错误信息
fn choose_default_agent(state: &State) -> Result<String, String> {
    // 优先使用配置中指定的默认代理
    if let Some(name) = state.default_agent.as_deref() {
        let Some(agent) = state.agents.get(name) else {
            return Err(format!("default agent \"{}\" not found", name));
        };
        // 验证代理是否可用
        if agent.definition.mode == "subagent" {
            return Err(format!("default agent \"{}\" is a subagent", name));
        }
        if agent.definition.hidden {
            return Err(format!("default agent \"{}\" is hidden", name));
        }
        return Ok(name.to_string());
    }

    // 尝试使用内置的首选代理
    for key in ["main"] {
        if let Some(agent) = state.agents.get(key) {
            if is_primary_visible(&agent.definition) {
                return Ok(key.to_string());
            }
        }
    }

    // 回退：按名称排序选择第一个可见的主代理
    let mut candidates = state
        .agents
        .iter()
        .filter(|(_, agent)| is_primary_visible(&agent.definition))
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next().ok_or_else(|| "no primary visible agent found".to_string())
}

/// 获取代理生成提示词
///
/// 返回用于生成代理配置的提示词模板。
///
/// # 返回值
///
/// 静态字符串切片，包含生成提示词内容
pub fn generate_prompt() -> &'static str {
    PROMPT_GENERATE
}

/// 生成工具输出截断的 glob 模式
///
/// 构建用于匹配工具输出目录中所有文件的 glob 模式。
///
/// # 返回值
///
/// 返回 glob 模式字符串，如 "/path/to/data/tool-output/*"
fn truncate_glob() -> String {
    global::paths().data.join("tool-output").join("*").to_string_lossy().to_string()
}

fn normalize_agent_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "primary" => "primary".to_string(),
        "subagent" | "sub_agent" | "sub-agent" => "subagent".to_string(),
        _ => "all".to_string(),
    }
}

pub async fn permission_rules(name: &str) -> Option<Ruleset> {
    load_state().await.ok()?.agents.get(name).map(|agent| agent.permission.clone())
}

fn load_agent_configs(cfg: &Value) -> HashMap<String, DelegateAgentConfig> {
    let configured = cfg
        .get("agents")
        .cloned()
        .and_then(|value| {
            serde_json::from_value::<HashMap<String, DelegateAgentConfig>>(value).ok()
        })
        .unwrap_or_default();
    merged_agent_configs(&configured)
}

pub fn resolve_model_ref(
    config: &DelegateAgentConfig,
) -> Option<vw_shared::message::types::ModelRef> {
    let provider_id = config.provider.trim();
    let model_id = config.model.trim();
    if !provider_id.is_empty() && !model_id.is_empty() {
        return Some(vw_shared::message::types::ModelRef {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        });
    }

    let combined = model_id;
    if combined.is_empty() || !combined.contains('/') {
        return None;
    }

    let parsed = provider::parse_model(combined);
    Some(vw_shared::message::types::ModelRef {
        provider_id: parsed.provider_id,
        model_id: parsed.model_id,
    })
}

fn build_resolved_agent(
    key: &str,
    mut config: DelegateAgentConfig,
    defaults: &Ruleset,
    user: &Ruleset,
) -> ResolvedAgentDefinition {
    config.mode = normalize_agent_mode(&config.mode);
    if config.system_prompt.is_none() {
        config.system_prompt = match key {
            "researcher" => Some(PROMPT_RESEARCHER.to_string()),
            "coder" => Some(PROMPT_CODER.to_string()),
            "reviewer" => Some(PROMPT_REVIEWER.to_string()),
            "explore" => Some(PROMPT_EXPLORE.to_string()),
            _ => None,
        };
    }

    let selected_skills = config
        .allowed_skills
        .iter()
        .map(|name| permission_next::Rule {
            permission: "skill".to_string(),
            pattern: name.clone(),
            action: permission_next::Action::Allow,
        })
        .collect::<Ruleset>();

    ResolvedAgentDefinition {
        permission: permission_next::merge(&[
            defaults.clone(),
            permission_next::from_config(&config.permission),
            selected_skills,
            user.clone(),
        ]),
        definition: config,
    }
}

fn merge_json_value(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, source_value) in source {
                match target.get_mut(key) {
                    Some(target_value) => merge_json_value(target_value, source_value),
                    None => {
                        target.insert(key.clone(), source_value.clone());
                    }
                }
            }
        }
        (target, source) => *target = source.clone(),
    }
}

/// 加载代理状态
///
/// 从应用配置加载所有代理定义，并编译出运行时权限状态。
/// 此函数是代理状态初始化的核心逻辑。
///
/// # 处理流程
///
/// 1. 加载应用配置
/// 2. 解析用户权限配置
/// 3. 获取技能目录列表
/// 4. 构建默认权限规则
/// 5. 从统一 `agents` 配置生成 agent 列表
/// 6. 为需要动态路径的 agent 叠加运行时字段
/// 7. 确保所有代理都有工具输出目录的访问权限
/// 8. 返回完整状态
///
/// # 返回值
///
/// 返回初始化完成的 `State` 结构体
async fn load_state() -> Result<State, String> {
    let config = crate::config::get_current().await.map_err(|error| error.to_string())?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    // 桌面偏好只提供兼容权限字段；代理定义必须来自租户/项目配置。
    let mut cfg = tokio::task::spawn_blocking(move || {
        crate::session::ui_config::load_app_config_result(&tenant)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;
    cfg["agents"] = serde_json::to_value(config.agents).map_err(|error| error.to_string())?;
    let configured_agents = load_agent_configs(&cfg);

    // 解析用户权限配置
    let user = permission_next::from_config(cfg.get("permission").unwrap_or(&Value::Null));

    // 获取技能目录，用于设置外部目录权限
    let skill_dirs = skill::dirs().await;

    // 构建外部目录权限映射
    let mut external_directory = Map::new();
    // 默认：所有外部目录需要询问
    external_directory.insert("*".to_string(), Value::String("ask".to_string()));
    // 工具输出目录：允许访问
    external_directory.insert(truncate_glob(), Value::String("allow".to_string()));
    // 技能目录：允许访问
    for dir in skill_dirs {
        let pat = PathBuf::from(dir).join("*").to_string_lossy().to_string();
        external_directory.insert(pat, Value::String("allow".to_string()));
    }

    // 构建默认权限规则
    let defaults = permission_next::from_config(&json!({
        "*": "allow",                    // 默认允许所有操作
        "doom_loop": "ask",              // doom_loop 需要询问
        "external_directory": external_directory,  // 外部目录权限
        "question": "deny",              // 默认禁止问题工具
        "AskUserQuestion": "deny",       // Claude 风格问题工具名
        "read": {
            "*": "allow",                // 允许读取所有文件
            "*.env": "ask",              // .env 文件需要询问
            "*.env.*": "ask",            // .env.* 文件需要询问
            "*.env.example": "allow"     // .env.example 允许读取
        }
    }));

    let mut agents: HashMap<String, ResolvedAgentDefinition> = HashMap::new();

    for (key, config) in configured_agents {
        if !config.enabled {
            continue;
        }
        agents.insert(key.clone(), build_resolved_agent(&key, config, &defaults, &user));
    }

    // 确保所有代理都有工具输出目录的访问权限
    let trunc = truncate_glob();
    for agent in agents.values_mut() {
        // 检查是否已显式配置了拒绝规则
        let explicit = agent.permission.iter().any(|r| {
            r.permission == "external_directory"
                && r.action == permission_next::Action::Deny
                && r.pattern == trunc
        });
        if explicit {
            continue; // 如果已显式拒绝，跳过
        }
        // 添加允许规则
        agent.permission.push(permission_next::Rule {
            permission: "external_directory".to_string(),
            pattern: trunc.clone(),
            action: permission_next::Action::Allow,
        });
    }

    // 获取配置中的默认代理
    let default_agent = cfg.get("default_agent").and_then(Value::as_str).map(|s| s.to_string());

    Ok(State { agents, default_agent })
}
