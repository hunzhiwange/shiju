use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HooksConfig {
    /// 是否启用生命周期 hook 执行。
    ///
    /// hook 与主运行时在同一进程内执行，并共享相同权限。
    /// 已启用的 hook 处理器应保持最小职责并易于审计。
    #[serde(default = "default_hooks_enabled")]
    pub enabled: bool,
    /// 内置 hook 的开关配置。
    #[serde(default)]
    pub builtin: BuiltinHooksConfig,
    /// 插件 hook 的开关配置。
    #[serde(default)]
    pub plugins: PluginHooksConfig,
    /// Required handler identities. Missing or untrusted handlers reject execution.
    #[serde(default)]
    pub required: Vec<String>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            builtin: BuiltinHooksConfig::default(),
            plugins: PluginHooksConfig::default(),
            required: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct BuiltinHooksConfig {
    /// 是否启用 command-logger hook，用于记录工具调用以便审计。
    pub command_logger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginHooksConfig {
    /// 是否允许从已启用插件加载 hook 清单。
    pub enabled: bool,
}

impl Default for PluginHooksConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_hooks_enabled() -> bool {
    true
}
#[cfg(test)]
#[path = "hooks_tests.rs"]
mod hooks_tests;
