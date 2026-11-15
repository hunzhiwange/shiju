//! 为系统设置建立稳定的搜索条目、拼音匹配与页面跳转目标。




use config_ui::DesktopLocale;

use crate::app::App;
use crate::app::components::system_settings::{
    SystemTab,
    general_fields::{self, GeneralSettingField},
};
use crate::app::i18n::I18n;
use crate::app::state::{
    AgentConfigTab, AutonomySettingsTab, ChannelsSettingsTab, CronSettingsTab, GatewaySettingsTab,
    GeneralSettingsTab, McpSettingsTab, MemorySettingsTab, ModelConfigTab, SecuritySettingsTab,
    SkillsSettingsTab,
};

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
    pub(crate) fn page(self) -> SystemTab {
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

/// 单个设置页签共享的界面标签与搜索关键词。
pub use desktop_settings_security::SettingsTabDescriptor;

pub type SystemSettingsSearchEntry = desktop_settings_network::system_settings_search::SystemSettingsSearchEntry<SystemSettingsSearchTarget>;
use desktop_settings_network::system_settings_search::literal_search_forms;

/// 按标签、关键词、上级路径的命中顺序显示可跳转的设置页。
pub(super) fn search_results(app: &App, query: &str) -> Vec<SystemSettingsSearchEntry> {
    let forms = literal_search_forms(query);
    if forms.is_empty() {
        return Vec::new();
    }

    let mut matches: Vec<_> = catalog(app)
        .into_iter()
        .filter_map(|entry| entry.match_rank(&forms).map(|rank| (rank, entry)))
        .collect();
    matches.sort_by_key(|&(rank, entry)| {
        (rank, !matches!(entry.target, SystemSettingsSearchTarget::GeneralField(_)))
    });
    matches.into_iter().map(|(_, entry)| entry).collect()
}

pub(super) fn navigation_label(locale: DesktopLocale, tab: SystemTab) -> &'static str {
    match tab {
        SystemTab::Memory => crate::app::i18n::settings_runtime::text(locale, "记忆系统配置"),
        SystemTab::Runtime => crate::app::i18n::settings_runtime::text(locale, "运行时配置"),
        SystemTab::Observability => {
            crate::app::i18n::settings_runtime::text(locale, "可观测性配置")
        }
        SystemTab::Security => crate::app::i18n::settings_security::text(locale, "安全配置"),
        _ => connection_target_label(locale, SystemSettingsSearchTarget::TopLevel(tab))
            .unwrap_or_else(|| I18n::new(locale).__(tab.label())),
    }
}

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
    let i18n = I18n::new(app.desktop_locale);
    for entry in &mut entries {
        use crate::app::i18n::settings_runtime::text as runtime_text;
        use crate::app::i18n::settings_security::text as security_text;
        match entry.target {
            SystemSettingsSearchTarget::Memory(tab) => {
                entry.label = system_settings_memory::memory_tab_label(app.desktop_locale, tab);
                entry.parent_label = Some(runtime_text(app.desktop_locale, "记忆系统配置"));
            }
            SystemSettingsSearchTarget::Security(tab) => {
                entry.label = system_settings_security::security_tab_label(app.desktop_locale, tab);
                entry.parent_label = Some(security_text(app.desktop_locale, "安全配置"));
            }
            SystemSettingsSearchTarget::TopLevel(tab) => {
                entry.label = navigation_label(app.desktop_locale, tab);
            }
            SystemSettingsSearchTarget::Agents(crate::app::state::AgentConfigTab::Runtime) => {
                entry.label = runtime_text(app.desktop_locale, "运行时配置");
                entry.parent_label = entry.original_parent_label.map(|label| i18n.__(label));
            }
            _ => {
                entry.label = i18n.__(entry.original_label);
                entry.parent_label = entry.original_parent_label.map(|label| i18n.__(label));
            }
        }

        if !matches!(entry.target, SystemSettingsSearchTarget::TopLevel(_))
            && let Some(label) = connection_target_label(app.desktop_locale, entry.target)
        {
            entry.label = label;
            use crate::app::i18n::settings_models::{self, connection_text::text};
            entry.parent_label = match entry.target {
                SystemSettingsSearchTarget::Models(_) => {
                    Some(settings_models::text(app.desktop_locale, "模型"))
                }
                SystemSettingsSearchTarget::Gateway(_) => {
                    Some(text(app.desktop_locale, "服务端网关配置"))
                }
                SystemSettingsSearchTarget::Network(_) => {
                    Some(text(app.desktop_locale, "网络配置"))
                }
                _ => entry.parent_label,
            };
        }
    }
    for descriptor in system_settings_general::GENERAL_TABS {
        entries.extend(general_fields::ALL.iter().filter(|field| field.tab == descriptor.tab).map(
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
        ));
    }
    entries
}

fn connection_target_label(
    locale: DesktopLocale,
    target: SystemSettingsSearchTarget,
) -> Option<&'static str> {
    use crate::app::i18n::settings_models::{self};
    use SystemSettingsSearchTarget::{Gateway, Models, Network, TopLevel};
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
