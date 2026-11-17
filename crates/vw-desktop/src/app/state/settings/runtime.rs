
/// 技能设置面板状态
///
/// 管理技能系统的配置，包括开放技能目录和提示注入模式。
pub use desktop_settings_skills::types::SkillsSettingsTab;


pub(crate) use desktop_settings_skills::update::state::State as SkillsSettingsState;

pub(crate) use desktop_settings_network::app_init::state::{WebSearchSettingsState};

pub(crate) use desktop_settings_network::state::{BrowserSettingsState, GatewaySettingsTab, GatewaySettingsState, GatewayClientSettingsState};

pub(crate) use desktop_settings_runtime::state::CostSettingsState;

pub use desktop_settings_memory::state::MemorySettingsTab;

pub(crate) use desktop_settings_memory::state::MemorySettingsState;





pub(crate) use desktop_settings_runtime::state::{HooksSettingsState, RuntimeSettingsState, ReliabilitySettingsState};

#[cfg(test)]
use super::*;
#[cfg(test)]
pub use desktop_settings_skills::types::PluginCatalogItem;
#[cfg(test)]
pub use desktop_settings_skills::types::SkillsCatalogItem;
#[cfg(test)]
pub use desktop_settings_skills::types::SkillsCatalogKind;
#[cfg(test)]
pub use desktop_settings_skills::types::SkillsDirectoryScope;
#[cfg(test)]
pub use desktop_settings_skills::types::SkillsSelectedDetail;
#[cfg(test)]
pub(crate) use desktop_settings_network::state::GatewayClientServerDraft;
#[cfg(test)]
pub(crate) use desktop_settings_runtime::state::CostPriceInput;
