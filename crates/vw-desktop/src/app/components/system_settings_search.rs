//! 为系统设置建立稳定的搜索条目、拼音匹配与页面跳转目标。

use crate::app::App;
use crate::app::components::system_settings::SystemTab;

pub use desktop_settings_shell::search::SystemSettingsSearchTarget;

pub type SystemSettingsSearchEntry =
    desktop_settings_network::system_settings_search::SystemSettingsSearchEntry<
        SystemSettingsSearchTarget,
    >;

/// 按标签、关键词、上级路径的命中顺序显示可跳转的设置页。
pub(super) fn search_results(app: &App, query: &str) -> Vec<SystemSettingsSearchEntry> {
    desktop_settings_shell::search::search_results(catalog(app), query)
}

pub(super) use desktop_settings_shell::search::navigation_label;

/// 根据当前应用可到达的设置页面构建搜索目录。
pub fn catalog(app: &App) -> Vec<SystemSettingsSearchEntry> {
    use crate::app::components::{
        system_settings_agents, system_settings_autonomy, system_settings_channels,
        system_settings_cron, system_settings_gateway, system_settings_general,
        system_settings_mcp, system_settings_memory, system_settings_models,
        system_settings_network, system_settings_security, system_settings_skills,
    };

    let mut entries = SystemTab::all()
        .map(|tab| {
            SystemSettingsSearchEntry::new(
                tab.label(),
                None,
                super::system_settings::system_tab_keywords(tab),
                SystemSettingsSearchTarget::TopLevel(tab),
            )
        })
        .collect::<Vec<_>>();

    macro_rules! extend_tabs {
        ($descriptors:expr, $parent:expr, $target:ident) => {
            entries.extend($descriptors.iter().map(|descriptor| {
                SystemSettingsSearchEntry::new(
                    descriptor.label,
                    Some($parent),
                    descriptor.keywords,
                    SystemSettingsSearchTarget::$target(descriptor.tab),
                )
            }));
        };
    }

    extend_tabs!(system_settings_general::GENERAL_TABS, "常规设置", General);
    extend_tabs!(system_settings_models::MODEL_TABS, "模型", Models);
    extend_tabs!(system_settings_cron::CRON_TABS, "定时任务", Cron);
    extend_tabs!(system_settings_agents::AGENT_CONFIG_TABS, "智能体", Agents);
    let channel_tabs = system_settings_channels::channel_tab_descriptors();
    extend_tabs!(channel_tabs, "智能体 / 通道", Channels);
    extend_tabs!(system_settings_memory::MEMORY_TABS, "记忆配置", Memory);
    if app.memory_settings.backend == "qdrant" {
        extend_tabs!(system_settings_memory::QDRANT_TAB, "记忆配置", Memory);
    }
    extend_tabs!(system_settings_autonomy::AUTONOMY_TABS, "自治配置", Autonomy);
    extend_tabs!(system_settings_security::SECURITY_TABS, "安全配置", Security);
    extend_tabs!(system_settings_gateway::GATEWAY_TABS, "服务端网关", Gateway);
    extend_tabs!(system_settings_network::NETWORK_TABS, "网络配置", Network);
    extend_tabs!(system_settings_mcp::MCP_TABS, "智能体 / MCP", AgentMcp);
    extend_tabs!(system_settings_skills::SKILLS_TABS, "智能体 / 技能", AgentSkills);
    desktop_settings_shell::search::localize_catalog(entries, app.desktop_locale)
}
