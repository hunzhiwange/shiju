//! 为系统设置建立稳定的搜索条目、拼音匹配与页面跳转目标。

use std::borrow::Cow;

use pinyin::ToPinyin;
use vw_config_types::ui::DesktopLocale;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsTabDescriptor<T> {
    /// 跳转时选中的强类型页签。
    pub tab: T,
    /// 在搜索结果中展示的页签名称。
    pub label: &'static str,
    /// 不直接展示、仅用于召回的同义关键词。
    pub keywords: &'static [&'static str],
}

impl<T> SettingsTabDescriptor<T> {
    pub const fn new(tab: T, label: &'static str, keywords: &'static [&'static str]) -> Self {
        Self { tab, label, keywords }
    }
}

/// 一个可导航系统设置目标的搜索元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemSettingsSearchEntry {
    original_label: &'static str,
    original_parent_label: Option<&'static str>,
    ancestor_label: Option<&'static str>,
    /// 搜索结果的主标签。
    pub label: &'static str,
    /// 可选的上级设置分区标签。
    pub parent_label: Option<&'static str>,
    /// 补充检索词，不参与界面展示。
    pub keywords: &'static [&'static str],
    /// 命中后使用的强类型导航目标。
    pub target: SystemSettingsSearchTarget,
}

impl SystemSettingsSearchEntry {
    pub const fn new(
        label: &'static str,
        parent_label: Option<&'static str>,
        keywords: &'static [&'static str],
        target: SystemSettingsSearchTarget,
    ) -> Self {
        Self {
            label,
            original_label: label,
            original_parent_label: parent_label,
            ancestor_label: None,
            parent_label,
            keywords,
            target,
        }
    }

    /// 使用文字、拼音全拼和首字母形式匹配查询。空查询匹配全部条目。
    pub fn matches(&self, query: &str) -> bool {
        let forms = literal_search_forms(query);
        forms.is_empty() || self.match_rank(&forms).is_some()
    }

    pub(super) fn parent_path(&self) -> Option<Cow<'static, str>> {
        let parent = self.parent_label?;
        Some(match self.ancestor_label {
            Some(ancestor) => Cow::Owned(format!("{parent} > {ancestor}")),
            None => Cow::Borrowed(parent),
        })
    }

    fn match_rank(&self, query_forms: &[String]) -> Option<u8> {
        let contains_query =
            |candidate: &str| query_forms.iter().any(|query| candidate.contains(query));
        let label_rank = [self.label, self.original_label]
            .into_iter()
            .filter_map(|label| {
                let forms = search_forms(label);
                if forms.iter().any(|candidate| query_forms.contains(candidate)) {
                    Some(0)
                } else {
                    forms.iter().any(|candidate| contains_query(candidate)).then_some(1)
                }
            })
            .min();
        let matches = |text| search_forms(text).iter().any(|form| contains_query(form));
        if label_rank.is_some() {
            label_rank
        } else if self.keywords.iter().copied().any(matches) {
            Some(2)
        } else if self.ancestor_label.is_none()
            && self.parent_label.into_iter().chain(self.original_parent_label).any(matches)
        {
            Some(3)
        } else {
            None
        }
    }
}

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

fn literal_search_forms(text: &str) -> Vec<String> {
    let literal = normalized_literal(text);
    if literal.is_empty() {
        return Vec::new();
    }

    let compact = literal.replace(' ', "");
    if compact == literal { vec![literal] } else { vec![literal, compact] }
}

fn search_forms(text: &str) -> Vec<String> {
    let mut forms = literal_search_forms(text);
    let mut full_pinyin = String::new();
    let mut spaced_pinyin = String::new();
    let mut initials = String::new();

    for character in text.to_lowercase().chars() {
        if let Some(pinyin) = character.to_pinyin() {
            let plain = pinyin.plain();
            full_pinyin.push_str(plain);
            push_token(&mut spaced_pinyin, plain);
            if let Some(initial) = plain.chars().next() {
                initials.push(initial);
            }
        } else if character.is_alphanumeric() {
            full_pinyin.push(character);
            spaced_pinyin.push(character);
            initials.push(character);
        } else if !spaced_pinyin.ends_with(' ') && !spaced_pinyin.is_empty() {
            spaced_pinyin.push(' ');
        }
    }

    push_unique(&mut forms, full_pinyin);
    push_unique(&mut forms, spaced_pinyin.trim().to_owned());
    push_unique(&mut forms, initials);
    forms
}

fn normalized_literal(text: &str) -> String {
    let mut normalized = String::new();
    let mut pending_space = false;

    for character in text.to_lowercase().chars() {
        if character.is_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(character);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }

    normalized
}

fn push_token(output: &mut String, token: &str) {
    if !output.is_empty() && !output.ends_with(' ') {
        output.push(' ');
    }
    output.push_str(token);
}

fn push_unique(forms: &mut Vec<String>, form: String) {
    if !form.is_empty() && !forms.contains(&form) {
        forms.push(form);
    }
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
