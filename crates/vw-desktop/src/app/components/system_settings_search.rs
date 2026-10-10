//! 为系统设置建立稳定的搜索条目、拼音匹配与页面跳转目标。

use pinyin::ToPinyin;

use crate::app::App;
use crate::app::components::system_settings::SystemTab;
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
        Self { label, parent_label, keywords, target }
    }

    /// 使用文字、拼音全拼和首字母形式匹配查询；空查询匹配全部条目。
    pub fn matches(&self, query: &str) -> bool {
        let query_forms = literal_search_forms(query);
        if query_forms.is_empty() {
            return true;
        }

        self.searchable_texts().any(|text| {
            let candidate_forms = search_forms(text);
            query_forms.iter().any(|query_form| {
                candidate_forms.iter().any(|candidate| candidate.contains(query_form))
            })
        })
    }

    fn searchable_texts(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.label).chain(self.parent_label).chain(self.keywords.iter().copied())
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
        .into_iter()
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
