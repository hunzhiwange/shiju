//! 代理配置工具
//!
//! 动态管理运行时代理设置，支持设置、查看和清除代理配置。

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::{
    self, Config, ProxyConfig, ProxyScope, apply_proxy_to_process_env, clear_proxy_env,
    get_project_without_env_blocking, runtime_proxy_config, set_runtime_proxy_config,
    validate_proxy_config,
};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::util::MaybeSet;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// 代理配置工具结构体
///
/// 提供代理配置的动态管理功能，支持多种操作：
/// - 获取当前代理配置（get）
/// - 设置代理配置（set）
/// - 禁用代理（disable）
/// - 列出支持的服务（list_services）
/// - 应用环境变量（apply_env）
/// - 清除环境变量（clear_env）
///
/// # 字段说明
///
/// * `config` - 应用配置的共享引用，包含配置文件路径和工作目录等信息
/// * `security` - 安全策略的共享引用，用于权限控制和动作限制
///
/// # 示例
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use crate::app::agent::tools::proxy_config::ProxyConfigTool;
/// use crate::app::agent::config::Config;
/// use crate::app::agent::security::SecurityPolicy;
///
/// let config = Arc::new(Config::default());
/// let security = Arc::new(SecurityPolicy::default());
/// let tool = ProxyConfigTool::new(config, security);
/// ```
pub struct ProxyConfigTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl ProxyConfigTool {
    /// 创建新的代理配置工具实例
    ///
    /// # 参数
    ///
    /// * `config` - 应用配置的共享引用
    /// * `security` - 安全策略的共享引用
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `ProxyConfigTool` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let tool = ProxyConfigTool::new(config, security);
    /// ```
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    /// 从数据库基础配置与项目覆盖加载配置（不包含环境变量）
    ///
    /// 读取本地租户基础配置并合并项目覆盖。
    ///
    /// # 返回值
    ///
    /// - `Ok(Config)` - 成功加载并解析的配置对象
    /// - `Err(anyhow::Error)` - 读取或解析失败时返回错误
    ///
    /// # 错误
    ///
    /// - 共享数据库读取失败
    /// - 项目覆盖解析或校验失败
    fn load_config_without_env(&self) -> anyhow::Result<Config> {
        get_project_without_env_blocking(&self.config.workspace_dir)
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    /// 检查写操作权限
    ///
    /// 验证是否允许执行写操作，检查自主权限和速率限制
    ///
    /// # 返回值
    ///
    /// - `Some(ToolResult)` - 权限被拒绝，返回包含错误信息的工具结果
    /// - `None` - 权限检查通过，允许执行操作
    ///
    /// # 权限检查
    ///
    /// 1. 检查是否处于只读模式（`can_act()`）
    /// 2. 检查是否超过速率限制（`record_action()`）
    fn require_write_access(&self) -> Option<ToolResult> {
        // 检查是否允许执行操作（非只读模式）
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 检查速率限制并记录动作
        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        None
    }

    /// 解析代理范围字符串
    ///
    /// 将字符串转换为 ProxyScope 枚举值，支持多种别名
    ///
    /// # 参数
    ///
    /// * `raw` - 原始范围字符串，不区分大小写
    ///
    /// # 返回值
    ///
    /// - `Some(ProxyScope)` - 成功解析的范围值
    /// - `None` - 无法识别的范围字符串
    ///
    /// # 支持的范围
    ///
    /// | 范围 | 支持的别名 |
    /// |------|-----------|
    /// | Environment | "environment", "env" |
    /// | Vibewindow | "vibewindow", "internal", "core" |
    /// | Services | "services", "service" |
    fn parse_scope(raw: &str) -> Option<ProxyScope> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "environment" | "env" => Some(ProxyScope::Environment),
            "vibewindow" | "internal" | "core" => Some(ProxyScope::Vibewindow),
            "services" | "service" => Some(ProxyScope::Services),
            _ => None,
        }
    }

    /// 解析字符串列表
    ///
    /// 从 JSON 值中解析字符串列表，支持逗号分隔的字符串或字符串数组
    ///
    /// # 参数
    ///
    /// * `raw` - JSON 值，可以是字符串或字符串数组
    /// * `field` - 字段名，用于错误信息
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<String>)` - 成功解析的字符串列表（已去除空白和空项）
    /// - `Err(anyhow::Error)` - 格式不正确时返回错误
    ///
    /// # 支持的格式
    ///
    /// - 字符串："item1, item2, item3"
    /// - 数组：["item1", "item2", "item3"]
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let list = ProxyConfigTool::parse_string_list(&json!("a, b, c"), "test")?;
    /// // 返回：["a", "b", "c"]
    /// ```
    fn parse_string_list(raw: &Value, field: &str) -> anyhow::Result<Vec<String>> {
        // 处理逗号分隔的字符串格式
        if let Some(raw_string) = raw.as_str() {
            return Ok(raw_string
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(ToOwned::to_owned)
                .collect());
        }

        // 处理数组格式
        if let Some(array) = raw.as_array() {
            let mut out = Vec::new();
            for item in array {
                let value = item
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("'{field}' array must only contain strings"))?;
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
            return Ok(out);
        }

        anyhow::bail!("'{field}' must be a string or string[]")
    }

    /// 解析可选的字符串更新值
    ///
    /// 从参数中提取字符串字段，支持 null 值表示清除该字段
    ///
    /// # 参数
    ///
    /// * `args` - JSON 参数对象
    /// * `field` - 要提取的字段名
    ///
    /// # 返回值
    ///
    /// - `MaybeSet::Set(String)` - 字段存在且非空，返回设置的值
    /// - `MaybeSet::Null` - 字段为 null 或空字符串，表示要清除
    /// - `MaybeSet::Unset` - 字段不存在，保持原值不变
    ///
    /// # 错误
    ///
    /// 如果字段存在但不是字符串或 null，返回错误
    fn parse_optional_string_update(args: &Value, field: &str) -> anyhow::Result<MaybeSet<String>> {
        // 字段不存在，表示不更新
        let Some(raw) = args.get(field) else {
            return Ok(MaybeSet::Unset);
        };

        // 字段为 null，表示要清除
        if raw.is_null() {
            return Ok(MaybeSet::Null);
        }

        // 提取字符串值并去除首尾空白
        let value = raw
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'{field}' must be a string or null"))?
            .trim()
            .to_string();

        // 空字符串等同于 null
        let output = if value.is_empty() { MaybeSet::Null } else { MaybeSet::Set(value) };
        Ok(output)
    }

    /// 获取当前进程环境变量的快照
    ///
    /// 读取标准代理相关环境变量的当前值
    ///
    /// # 返回值
    ///
    /// 返回包含以下环境变量的 JSON 对象：
    /// - `HTTP_PROXY` - HTTP 代理地址
    /// - `HTTPS_PROXY` - HTTPS 代理地址
    /// - `ALL_PROXY` - 通用代理地址
    /// - `NO_PROXY` - 不使用代理的地址列表
    fn env_snapshot() -> Value {
        json!({
            "HTTP_PROXY": std::env::var("HTTP_PROXY").ok(),
            "HTTPS_PROXY": std::env::var("HTTPS_PROXY").ok(),
            "ALL_PROXY": std::env::var("ALL_PROXY").ok(),
            "NO_PROXY": std::env::var("NO_PROXY").ok(),
        })
    }

    /// 将代理配置转换为 JSON 格式
    ///
    /// # 参数
    ///
    /// * `proxy` - 代理配置引用
    ///
    /// # 返回值
    ///
    /// 返回包含代理配置完整信息的 JSON 对象，包括：
    /// - enabled: 是否启用
    /// - scope: 作用范围
    /// - http_proxy: HTTP 代理地址
    /// - https_proxy: HTTPS 代理地址
    /// - all_proxy: 通用代理地址
    /// - no_proxy: 排除列表
    /// - services: 服务选择器列表
    fn proxy_json(proxy: &ProxyConfig) -> Value {
        json!({
            "enabled": proxy.enabled,
            "scope": proxy.scope,
            "http_proxy": proxy.http_proxy,
            "https_proxy": proxy.https_proxy,
            "all_proxy": proxy.all_proxy,
            "no_proxy": proxy.normalized_no_proxy(),
            "services": proxy.normalized_services(),
        })
    }

    /// 处理获取代理配置的请求
    ///
    /// 返回文件配置、运行时配置和环境变量的完整快照
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)` - 成功返回包含以下信息的 JSON：
    ///   - proxy: 配置文件中的代理设置
    ///   - runtime_proxy: 运行时代理设置
    ///   - environment: 当前环境变量快照
    /// - `Err(anyhow::Error)` - 读取配置失败
    fn handle_get(&self) -> anyhow::Result<ToolResult> {
        let file_proxy = self.load_config_without_env()?.proxy;
        let runtime_proxy = runtime_proxy_config();
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "proxy": Self::proxy_json(&file_proxy),
                "runtime_proxy": Self::proxy_json(&runtime_proxy),
                "environment": Self::env_snapshot(),
            }))?,
            error: None,
        })
    }

    /// 处理列出支持服务的请求
    ///
    /// 返回所有支持的服务键和选择器列表，以及使用示例
    ///
    /// # 返回值
    ///
    /// 返回包含以下信息的 JSON：
    /// - supported_service_keys: 支持的服务键列表
    /// - supported_selectors: 支持的选择器列表
    /// - usage_example: 使用示例
    fn handle_list_services(&self) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "supported_service_keys": ProxyConfig::supported_service_keys(),
                "supported_selectors": ProxyConfig::supported_service_selectors(),
                "usage_example": {
                    "action": "set",
                    "scope": "services",
                    "services": ["provider.openai", "tool.http_request", "channel.telegram"]
                }
            }))?,
            error: None,
        })
    }

    /// 处理设置代理配置的请求
    ///
    /// 更新代理配置并同步到配置文件和运行时
    ///
    /// # 参数
    ///
    /// * `args` - JSON 参数对象，可包含以下字段：
    ///   - enabled: 是否启用代理（布尔值）
    ///   - scope: 代理范围（environment/vibewindow/services）
    ///   - http_proxy: HTTP 代理地址
    ///   - https_proxy: HTTPS 代理地址
    ///   - all_proxy: 通用代理地址
    ///   - no_proxy: 排除列表
    ///   - services: 服务选择器列表
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)` - 成功返回更新后的配置信息
    /// - `Err(anyhow::Error)` - 配置验证或保存失败
    ///
    /// # 行为说明
    ///
    /// - 如果未显式设置 enabled 但提供了代理 URL，则自动启用代理
    /// - 如果清除了所有代理 URL，则自动禁用代理
    /// - 如果 scope 为 Environment 且代理启用，会自动应用环境变量
    /// - 如果从 Environment 切换到其他 scope，会自动清除环境变量
    async fn handle_set(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let mut cfg = self.load_config_without_env()?;
        let before = cfg.clone();
        let previous_scope = cfg.proxy.scope;
        let mut proxy = cfg.proxy.clone();
        let mut touched_proxy_url = false;

        // 更新启用状态
        if let Some(enabled) = args.get("enabled") {
            proxy.enabled =
                enabled.as_bool().ok_or_else(|| anyhow::anyhow!("'enabled' must be a boolean"))?;
        }

        // 更新代理范围
        if let Some(scope_raw) = args.get("scope") {
            let scope =
                scope_raw.as_str().ok_or_else(|| anyhow::anyhow!("'scope' must be a string"))?;
            proxy.scope = Self::parse_scope(scope).ok_or_else(|| {
                anyhow::anyhow!("Invalid scope '{scope}'. Use environment|vibewindow|services")
            })?;
        }

        // 更新 HTTP 代理设置
        match Self::parse_optional_string_update(args, "http_proxy")? {
            MaybeSet::Set(update) => {
                proxy.http_proxy = Some(update);
                touched_proxy_url = true;
            }
            MaybeSet::Null => {
                proxy.http_proxy = None;
                touched_proxy_url = true;
            }
            MaybeSet::Unset => {}
        }

        // 更新 HTTPS 代理设置
        match Self::parse_optional_string_update(args, "https_proxy")? {
            MaybeSet::Set(update) => {
                proxy.https_proxy = Some(update);
                touched_proxy_url = true;
            }
            MaybeSet::Null => {
                proxy.https_proxy = None;
                touched_proxy_url = true;
            }
            MaybeSet::Unset => {}
        }

        // 更新通用代理设置
        match Self::parse_optional_string_update(args, "all_proxy")? {
            MaybeSet::Set(update) => {
                proxy.all_proxy = Some(update);
                touched_proxy_url = true;
            }
            MaybeSet::Null => {
                proxy.all_proxy = None;
                touched_proxy_url = true;
            }
            MaybeSet::Unset => {}
        }

        // 更新排除列表
        if let Some(no_proxy_raw) = args.get("no_proxy") {
            proxy.no_proxy = Self::parse_string_list(no_proxy_raw, "no_proxy")?;
            touched_proxy_url = true;
        }

        // 更新服务选择器列表
        if let Some(services_raw) = args.get("services") {
            proxy.services = Self::parse_string_list(services_raw, "services")?;
        }

        // 自动启用/禁用逻辑：
        // 当用户提供代理 URL 时自动启用代理，
        // 当在同一更新中清除所有代理 URL 时自动禁用
        if args.get("enabled").is_none() && touched_proxy_url {
            proxy.enabled = proxy.has_any_proxy_url();
        }

        // 标准化并验证配置
        proxy.no_proxy = proxy.normalized_no_proxy();
        proxy.services = proxy.normalized_services();
        validate_proxy_config(&proxy)?;

        // 保存到配置文件
        cfg.proxy = proxy.clone();
        config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;
        // 更新运行时配置
        set_runtime_proxy_config(proxy.clone());

        // 根据范围处理环境变量
        if proxy.enabled && proxy.scope == ProxyScope::Environment {
            apply_proxy_to_process_env(&proxy);
        } else if previous_scope == ProxyScope::Environment {
            clear_proxy_env();
        }

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Proxy configuration updated",
                "proxy": Self::proxy_json(&proxy),
                "environment": Self::env_snapshot(),
            }))?,
            error: None,
        })
    }

    /// 处理禁用代理的请求
    ///
    /// 禁用代理并可选地清除环境变量
    ///
    /// # 参数
    ///
    /// * `args` - JSON 参数对象，可包含：
    ///   - clear_env: 是否清除环境变量（布尔值，默认根据当前 scope 决定）
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)` - 成功返回禁用后的配置信息
    /// - `Err(anyhow::Error)` - 保存配置失败
    ///
    /// # 行为说明
    ///
    /// - 如果当前 scope 为 Environment，默认清除环境变量
    /// - 可通过 clear_env 参数覆盖默认行为
    async fn handle_disable(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let mut cfg = self.load_config_without_env()?;
        let before = cfg.clone();
        // 默认行为：如果当前 scope 为 Environment，则清除环境变量
        let clear_env_default = cfg.proxy.scope == ProxyScope::Environment;
        cfg.proxy.enabled = false;
        config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        // 更新运行时配置
        set_runtime_proxy_config(cfg.proxy.clone());

        // 根据参数或默认值决定是否清除环境变量
        let clear_env = args.get("clear_env").and_then(Value::as_bool).unwrap_or(clear_env_default);
        if clear_env {
            clear_proxy_env();
        }

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Proxy disabled",
                "proxy": Self::proxy_json(&cfg.proxy),
                "environment": Self::env_snapshot(),
            }))?,
            error: None,
        })
    }

    /// 处理应用环境变量的请求
    ///
    /// 将代理配置应用到进程环境变量中
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)` - 成功返回应用后的环境变量快照
    /// - `Err(anyhow::Error)` - 代理未启用或 scope 不是 Environment
    ///
    /// # 前置条件
    ///
    /// - 代理必须已启用（enabled = true）
    /// - 代理范围必须为 Environment
    fn handle_apply_env(&self) -> anyhow::Result<ToolResult> {
        let cfg = self.load_config_without_env()?;
        let proxy = cfg.proxy;
        validate_proxy_config(&proxy)?;

        // 验证代理是否已启用
        if !proxy.enabled {
            anyhow::bail!("Proxy is disabled. Use action 'set' with enabled=true first");
        }

        // 验证范围是否为 Environment
        if proxy.scope != ProxyScope::Environment {
            anyhow::bail!(
                "apply_env only works when proxy.scope is 'environment' (current: {:?})",
                proxy.scope
            );
        }

        // 应用环境变量并更新运行时配置
        apply_proxy_to_process_env(&proxy);
        set_runtime_proxy_config(proxy.clone());

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Proxy environment variables applied",
                "proxy": Self::proxy_json(&proxy),
                "environment": Self::env_snapshot(),
            }))?,
            error: None,
        })
    }

    /// 处理清除环境变量的请求
    ///
    /// 清除进程中的代理相关环境变量
    ///
    /// # 返回值
    ///
    /// 返回清除后的环境变量快照
    fn handle_clear_env(&self) -> anyhow::Result<ToolResult> {
        clear_proxy_env();
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Proxy environment variables cleared",
                "environment": Self::env_snapshot(),
            }))?,
            error: None,
        })
    }
}

/// Tool trait 实现
///
/// 为 ProxyConfigTool 实现 Tool trait，使其可作为工具被调用
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ProxyConfigTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 工具标识符："proxy_config"
    fn name(&self) -> &str {
        "proxy_config"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    ///
    /// 工具的中文描述，说明功能和支持的范围
    fn description(&self) -> &str {
        "管理 VibeWindow 代理设置（范围：environment | vibewindow | services），包括运行时和进程环境应用"
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具支持的所有参数及其类型和约束
    ///
    /// # 返回值
    ///
    /// JSON Schema 对象，包含以下参数定义：
    /// - action: 操作类型（get/set/disable/list_services/apply_env/clear_env）
    /// - enabled: 是否启用代理
    /// - scope: 代理范围
    /// - http_proxy: HTTP 代理地址
    /// - https_proxy: HTTPS 代理地址
    /// - all_proxy: 通用代理地址
    /// - no_proxy: 排除列表
    /// - services: 服务选择器列表
    /// - clear_env: 是否清除环境变量
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "set", "disable", "list_services", "apply_env", "clear_env"],
                    "default": "get"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Enable or disable proxy"
                },
                "scope": {
                    "type": "string",
                    "description": "Proxy scope: environment | vibewindow | services"
                },
                "http_proxy": {
                    "type": ["string", "null"],
                    "description": "HTTP proxy URL"
                },
                "https_proxy": {
                    "type": ["string", "null"],
                    "description": "HTTPS proxy URL"
                },
                "all_proxy": {
                    "type": ["string", "null"],
                    "description": "Fallback proxy URL for all protocols"
                },
                "no_proxy": {
                    "description": "Comma-separated string or array of NO_PROXY entries",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "services": {
                    "description": "Comma-separated string or array of service selectors used when scope=services",
                    "oneOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "clear_env": {
                    "type": "boolean",
                    "description": "When action=disable, clear process proxy environment variables"
                }
            }
        })
    }

    /// 执行工具操作
    ///
    /// 根据传入的参数执行相应的代理配置操作
    ///
    /// # 参数
    ///
    /// * `args` - JSON 参数对象，必须包含 action 字段指定操作类型
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)` - 操作执行结果（成功或失败）
    /// - `Err(anyhow::Error)` - 严重错误（通常会被转换为 ToolResult.error）
    ///
    /// # 操作路由
    ///
    /// - get: 获取当前配置（无需写权限）
    /// - list_services: 列出支持的服务（无需写权限）
    /// - set: 设置配置（需要写权限）
    /// - disable: 禁用代理（需要写权限）
    /// - apply_env: 应用环境变量（需要写权限）
    /// - clear_env: 清除环境变量（需要写权限）
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        // 提取并标准化 action 参数，默认为 "get"
        let action =
            args.get("action").and_then(Value::as_str).unwrap_or("get").to_ascii_lowercase();

        // 根据 action 路由到对应的处理函数
        let result = match action.as_str() {
            "get" => self.handle_get(),
            "list_services" => self.handle_list_services(),
            // 写操作需要权限检查
            "set" | "disable" | "apply_env" | "clear_env" => {
                if let Some(blocked) = self.require_write_access() {
                    return Ok(blocked);
                }

                match action.as_str() {
                    "set" => self.handle_set(&args).await,
                    "disable" => self.handle_disable(&args).await,
                    "apply_env" => self.handle_apply_env(),
                    "clear_env" => self.handle_clear_env(),
                    _ => unreachable!("handled above"),
                }
            }
            _ => anyhow::bail!(
                "Unknown action '{action}'. Valid: get, set, disable, list_services, apply_env, clear_env"
            ),
        };

        // 将错误转换为 ToolResult 格式，确保调用方总是得到 ToolResult
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
/// 测试代码位于 tests/proxy_config.rs 文件中
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
