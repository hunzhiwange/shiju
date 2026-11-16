use crate::{GeneralSettingsTab, SystemTab, general_fields::GeneralSettingField};
use desktop_settings_agents::agents_state::AgentConfigTab;
use desktop_settings_channels::state::ChannelsSettingsTab;
use desktop_settings_cron::state::CronSettingsTab;
use desktop_settings_mcp::state::McpSettingsTab;
use desktop_settings_memory::state::MemorySettingsTab;
use desktop_settings_network::state::GatewaySettingsTab;
use desktop_settings_providers::state::ModelConfigTab;
use desktop_settings_security::{
    app_init::state::AutonomySettingsTab, security_state::SecuritySettingsTab,
};
use desktop_settings_skills::types::SkillsSettingsTab;
/// 系统设置搜索结果对应的稳定强类型跳转目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemSettingsSearchTarget {
    TopLevel(SystemTab),
    General(GeneralSettingsTab),
    GeneralField(&'static GeneralSettingField),
    Models(ModelConfigTab),
    Cron(CronSettingsTab),
    Agents(AgentConfigTab),
    Channels(ChannelsSettingsTab),
    Memory(MemorySettingsTab),
    Autonomy(AutonomySettingsTab),
    Security(SecuritySettingsTab),
    Gateway(GatewaySettingsTab),
    Network(SystemTab),
    AgentMcp(McpSettingsTab),
    AgentSkills(SkillsSettingsTab),
}

impl SystemSettingsSearchTarget {
    pub fn page(self) -> SystemTab {
        match self {
            Self::TopLevel(tab) | Self::Network(tab) => tab,
            Self::General(_) | Self::GeneralField(_) => SystemTab::General,
            Self::Models(_) => SystemTab::Models,
            Self::Cron(_) => SystemTab::Cron,
            Self::Agents(_) | Self::Channels(_) | Self::AgentMcp(_) | Self::AgentSkills(_) => {
                SystemTab::Agents
            }
            Self::Memory(_) => SystemTab::Memory,
            Self::Autonomy(_) => SystemTab::Autonomy,
            Self::Security(_) => SystemTab::Security,
            Self::Gateway(_) => SystemTab::Gateway,
        }
    }
}

pub type SystemSettingsSearchEntry =
    desktop_settings_network::system_settings_search::SystemSettingsSearchEntry<
        SystemSettingsSearchTarget,
    >;
pub fn search_results(
    entries: Vec<SystemSettingsSearchEntry>,
    query: &str,
) -> Vec<SystemSettingsSearchEntry> {
    let forms = desktop_settings_network::system_settings_search::literal_search_forms(query);
    if forms.is_empty() {
        return Vec::new();
    }

    let mut matches: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| entry.match_rank(&forms).map(|rank| (rank, entry)))
        .collect();
    matches.sort_by_key(|&(rank, entry)| {
        (rank, !matches!(entry.target, SystemSettingsSearchTarget::GeneralField(_)))
    });
    matches.into_iter().map(|(_, entry)| entry).collect()
}
pub struct ActiveNavigation {
    pub page: SystemTab,
    pub general: GeneralSettingsTab,
    pub models: ModelConfigTab,
    pub cron: CronSettingsTab,
    pub agents: AgentConfigTab,
    pub channels: ChannelsSettingsTab,
    pub memory: MemorySettingsTab,
    pub autonomy: AutonomySettingsTab,
    pub security: SecuritySettingsTab,
    pub gateway: GatewaySettingsTab,
    pub mcp: McpSettingsTab,
    pub skills: SkillsSettingsTab,
}
pub fn search_target_is_active(
    state: &ActiveNavigation,
    target: SystemSettingsSearchTarget,
) -> bool {
    match target {
        SystemSettingsSearchTarget::GeneralField(_) => false,
        SystemSettingsSearchTarget::TopLevel(tab) => {
            tab == state.page || (tab == SystemTab::WebSearch && crate::is_network_tab(state.page))
        }
        SystemSettingsSearchTarget::General(tab) => {
            state.page == SystemTab::General && state.general == tab
        }
        SystemSettingsSearchTarget::Models(tab) => {
            state.page == SystemTab::Models && state.models == tab
        }
        SystemSettingsSearchTarget::Cron(tab) => state.page == SystemTab::Cron && state.cron == tab,
        SystemSettingsSearchTarget::Agents(tab) => {
            state.page == SystemTab::Agents && state.agents == tab
        }
        SystemSettingsSearchTarget::Channels(tab) => {
            state.page == SystemTab::Agents
                && state.agents == AgentConfigTab::Channels
                && state.channels == tab
        }
        SystemSettingsSearchTarget::Memory(tab) => {
            state.page == SystemTab::Memory && state.memory == tab
        }
        SystemSettingsSearchTarget::Autonomy(tab) => {
            state.page == SystemTab::Autonomy && state.autonomy == tab
        }
        SystemSettingsSearchTarget::Security(tab) => {
            state.page == SystemTab::Security && state.security == tab
        }
        SystemSettingsSearchTarget::Gateway(tab) => {
            state.page == SystemTab::Gateway && state.gateway == tab
        }
        SystemSettingsSearchTarget::Network(tab) => state.page == tab,
        SystemSettingsSearchTarget::AgentMcp(tab) => {
            state.page == SystemTab::Agents
                && state.agents == AgentConfigTab::Mcp
                && state.mcp == tab
        }
        SystemSettingsSearchTarget::AgentSkills(tab) => {
            state.page == SystemTab::Agents
                && state.agents == AgentConfigTab::Skills
                && state.skills == tab
        }
    }
}

use config_ui::DesktopLocale;
use desktop_i18n::I18n;
pub fn navigation_label(locale: DesktopLocale, tab: SystemTab) -> &'static str {
    match tab {
        SystemTab::Memory => desktop_i18n::settings_runtime::text(locale, "记忆系统配置"),
        SystemTab::Runtime => desktop_i18n::settings_runtime::text(locale, "运行时配置"),
        SystemTab::Observability => desktop_i18n::settings_runtime::text(locale, "可观测性配置"),
        SystemTab::Security => desktop_i18n::settings_security::text(locale, "安全配置"),
        _ => connection_target_label(locale, SystemSettingsSearchTarget::TopLevel(tab))
            .unwrap_or_else(|| I18n::new(locale).__(tab.label())),
    }
}

pub fn localize_catalog(
    mut entries: Vec<SystemSettingsSearchEntry>,
    locale: config_ui::DesktopLocale,
) -> Vec<SystemSettingsSearchEntry> {
    let i18n = I18n::new(locale);
    for entry in &mut entries {
        use desktop_i18n::settings_runtime::text as runtime_text;
        use desktop_i18n::settings_security::text as security_text;
        match entry.target {
            SystemSettingsSearchTarget::Memory(tab) => {
                entry.label =
                    desktop_settings_memory::system_settings_memory::memory_tab_label(locale, tab);
                entry.parent_label = Some(runtime_text(locale, "记忆系统配置"));
            }
            SystemSettingsSearchTarget::Security(tab) => {
                entry.label =
                    desktop_settings_security::system_settings_security::security_tab_label(
                        locale, tab,
                    );
                entry.parent_label = Some(security_text(locale, "安全配置"));
            }
            SystemSettingsSearchTarget::TopLevel(tab) => {
                entry.label = navigation_label(locale, tab);
            }
            SystemSettingsSearchTarget::Agents(AgentConfigTab::Runtime) => {
                entry.label = runtime_text(locale, "运行时配置");
                entry.parent_label = entry.original_parent_label.map(|label| i18n.__(label));
            }
            _ => {
                entry.label = i18n.__(entry.original_label);
                entry.parent_label = entry.original_parent_label.map(|label| i18n.__(label));
            }
        }

        if !matches!(entry.target, SystemSettingsSearchTarget::TopLevel(_))
            && let Some(label) = connection_target_label(locale, entry.target)
        {
            entry.label = label;
            use desktop_i18n::settings_models::{self, connection_text::text};
            entry.parent_label = match entry.target {
                SystemSettingsSearchTarget::Models(_) => {
                    Some(settings_models::text(locale, "模型"))
                }
                SystemSettingsSearchTarget::Gateway(_) => Some(text(locale, "服务端网关配置")),
                SystemSettingsSearchTarget::Network(_) => Some(text(locale, "网络配置")),
                _ => entry.parent_label,
            };
        }
    }
    for descriptor in desktop_settings_appearance::general::GENERAL_TABS {
        entries.extend(
            crate::general_fields::ALL.iter().filter(|field| field.tab == descriptor.tab).map(
                |&field| {
                    let mut entry = SystemSettingsSearchEntry::new(
                        field.label,
                        Some(i18n.__(descriptor.label)),
                        field.keywords,
                        SystemSettingsSearchTarget::GeneralField(field),
                    );
                    entry.label = i18n.__(field.label);
                    entry.ancestor_label = Some(i18n.__("常规"));
                    entry
                },
            ),
        );
    }
    entries
}

fn connection_target_label(
    locale: DesktopLocale,
    target: SystemSettingsSearchTarget,
) -> Option<&'static str> {
    use SystemSettingsSearchTarget::{Gateway, Models, Network, TopLevel};
    use desktop_i18n::settings_models::{self};
    use settings_models::connection_text::text;
    let key = match target {
        Models(tab) => {
            return Some(settings_models::text(
                locale,
                match tab {
                    ModelConfigTab::Models => "模型",
                    ModelConfigTab::ModelRoutes => "模型路由",
                    ModelConfigTab::QueryClassification => "模型分类",
                    ModelConfigTab::Multimodal => "多模态",
                },
            ));
        }
        Gateway(GatewaySettingsTab::Config) => "配置",
        Gateway(GatewaySettingsTab::Skeys) => "skey 管理",
        TopLevel(SystemTab::Providers) => "模型提供商",
        TopLevel(SystemTab::Models) => {
            return Some(settings_models::text(locale, "模型"));
        }
        TopLevel(SystemTab::Gateway) => "服务端网关配置",
        TopLevel(SystemTab::GatewayClient) => "客户端网关",
        Network(SystemTab::WebSearch) => "网页搜索",
        TopLevel(SystemTab::WebSearch) => "网络配置",
        Network(SystemTab::HttpRequest) | TopLevel(SystemTab::HttpRequest) => "网络请求",
        Network(SystemTab::Browser) | TopLevel(SystemTab::Browser) => "浏览器",
        Network(SystemTab::Proxy) | TopLevel(SystemTab::Proxy) => "代理",
        Network(SystemTab::Tunnel) | TopLevel(SystemTab::Tunnel) => "隧道",
        _ => return None,
    };
    Some(text(locale, key))
}
