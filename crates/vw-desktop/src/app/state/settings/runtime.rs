use super::*;

/// 技能设置面板状态
///
/// 管理技能系统的配置，包括开放技能目录和提示注入模式。
pub use desktop_settings_skills::types::{SkillsSettingsTab, SkillsDirectoryScope, SkillsCatalogKind, SkillsSelectedDetail, SkillsCatalogItem};

pub use desktop_settings_skills::types::PluginCatalogItem;

pub(crate) use desktop_settings_skills::update::state::State as SkillsSettingsState;

pub(crate) use desktop_settings_network::app_init::state::{WebSearchSettingsState};

pub(crate) use desktop_settings_network::state::{BrowserSettingsState, GatewaySettingsTab, GatewaySettingsState, GatewayClientServerDraft, GatewayClientSettingsState};

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct CostPriceInput {
    pub(crate) model: String,
    pub(crate) input_price: String,
    pub(crate) output_price: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CostSettingsState {
    pub(crate) enabled: bool,
    pub(crate) daily_limit_usd_input: String,
    pub(crate) monthly_limit_usd_input: String,
    pub(crate) warn_at_percent_input: String,
    pub(crate) allow_override: bool,
    pub(crate) prices: Vec<CostPriceInput>,
    pub(crate) show_help_modal: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for CostSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            daily_limit_usd_input: "10".to_string(),
            monthly_limit_usd_input: "100".to_string(),
            warn_at_percent_input: "80".to_string(),
            allow_override: false,
            prices: Vec::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

pub use desktop_settings_memory::state::MemorySettingsTab;

pub(crate) use desktop_settings_memory::state::MemorySettingsState;



#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;

pub(crate) use desktop_settings_runtime::state::{HooksSettingsState, RuntimeSettingsState, ReliabilitySettingsState};
