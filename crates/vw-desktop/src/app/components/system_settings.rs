//! # 系统设置模块
//!
//! 本模块提供系统设置面板的 UI 组件，用于展示和管理 VibeWindow 应用的各类配置选项。
//!
//! ## 主要功能
//!
//! - 提供统一的系统设置入口界面，包含多个配置分类标签页
//! - 支持通过标签页切换浏览不同类别的设置项
//! - 以模态对话框形式展示，支持点击外部区域关闭
//!
//! ## 配置分类
//!
//! 系统设置面板包含以下配置类别：
//!
//! - **常规设置 (General)**：界面、终端与编辑器配置
//! - **项目 (Projects)**：项目管理配置
//! - **提供商 (Providers)**：AI 服务提供商配置
//! - **模型 (Models)**：AI 模型配置
//! - **模型路由 (ModelRoutes)**：模式到模型的路由配置
//! - **查询模型分类 (QueryClassification)**：查询模型分类规则配置
//! - **心跳配置 (Heartbeat)**：心跳检测设置
//! - **定时任务配置 (Cron)**：定时任务配置
//! - **智能体配置 (Agents)**：智能体、对话流、快捷消息与心跳配置
//! - **员工配置 (Staff)**：任务看板员工名册、绑定和能力配置
//! - 智能体配置内的 **通道** 标签：CLI 与多通道消息集成配置
//! - **记忆配置 (Memory)**：记忆后端、缓存和嵌入检索配置
//! - 智能体配置内的 **运行时** 标签：native/docker/wasm 运行时配置
//! - **自治配置 (Autonomy)**：代理自治能力设置
//! - **安全配置 (Security)**：安全策略配置
//! - **网关配置 (Gateway)**：HTTP 网关、Auth 鉴权与 node-control 配置
//! - **可观测性配置 (Observability)**：日志与监控配置
//! - **钩子配置 (Hooks)**：运行时钩子配置
//! - **网络配置 (WebSearch / HttpRequest / Browser / Proxy / Tunnel)**：网页搜索、HTTP 请求、浏览器、代理与隧道配置

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    icon_btn, settings_close_button, settings_error_banner, settings_muted_text_style,
    settings_panel_style, settings_text_input_style,
};
use crate::app::components::system_settings_search::{
    SystemSettingsSearchEntry, SystemSettingsSearchTarget,
};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Vector};

/// 系统设置标签页枚举
///
/// 定义系统设置面板中所有可用的配置分类标签页。
/// 每个变体对应一个独立的配置模块，用户可通过点击标签页切换查看。
///
/// # 变体说明
///
/// - `General` - 常规设置，包含界面、终端与编辑器配置项
/// - `DialogueFlow` - 对话流设置，控制对话流程行为
/// - `Editor` - 兼容旧入口，选择时转入常规设置的编辑器标签
/// - `Projects` - 项目设置，管理项目相关配置
/// - `Providers` - 提供商设置，配置 AI 服务提供商
/// - `Models` - 模型设置，管理可用 AI 模型
/// - `ModelRoutes` - 模型路由设置，配置模式到模型的映射
/// - `QueryClassification` - 查询模型分类设置，配置匹配规则与模型路由目标
/// - `Heartbeat` - 心跳设置，配置心跳检测参数
/// - `Cron` - Cron 设置，配置定时任务
/// - `Agents` - Agents 设置，配置 researcher / coder / reviewer 等委托代理
/// - `Staff` - 员工配置，配置任务看板员工名册、绑定和能力
/// - `Reliability` - 可靠性设置，配置容错与恢复机制
/// - `Channels` - 通道设置，配置 CLI 与外部消息通道
/// - `Memory` - 记忆设置，配置记忆后端和向量检索参数
/// - `Runtime` - 运行时设置，配置执行环境与隔离选项
/// - `Acp` - ACP 设置，配置外部智能体后端
/// - `Mcp` - MCP 设置，配置全局 MCP 服务器
/// - `Autonomy` - 自治设置，配置代理自治能力
/// - `Security` - 安全设置，配置安全策略
/// - `Observability` - 可观测性设置，配置日志与监控
/// - `Storage` - 存储设置，配置持久化存储 provider 与连接参数
/// - `Proxy` - 代理设置，配置网络代理
/// - `Tunnel` - 隧道设置，配置网关公网暴露 provider
/// - `Skills` - 技能设置，配置代理可用技能
/// - `Hooks` - 钩子设置，配置运行时 hooks 与内置钩子
/// - `WebSearch` - Web 搜索设置，配置搜索 provider、凭据与请求参数
/// - `HttpRequest` - HTTP 请求设置，配置 http_request 工具白名单与网络限制
/// - `Browser` - 浏览器设置，配置 browser/browser_open 工具与后端
/// - `HttpRequest` - HTTP 请求设置，配置 http_request 工具白名单与请求限制
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTab {
    /// 常规设置标签页
    General,
    /// 兼容旧主题入口，选择时转入常规设置的主题标签
    Themes,
    /// 编辑器设置标签页
    Editor,
    /// 项目设置标签页
    Projects,
    /// 提供商设置标签页
    Providers,
    /// 模型设置标签页
    Models,
    /// 定时任务配置标签页
    Cron,
    /// 智能体配置标签页
    Agents,
    /// 员工配置标签页
    Staff,
    /// 通道配置标签页
    Channels,
    /// 记忆配置标签页
    Memory,
    /// 运行时配置标签页
    Runtime,
    /// 自治配置标签页
    Autonomy,
    /// 安全配置标签页
    Security,
    /// 客户端网关连接标签页
    GatewayClient,
    /// 网关配置标签页
    Gateway,
    /// 可观测性配置标签页
    Observability,
    /// 代理配置标签页
    Proxy,
    /// 隧道配置标签页
    Tunnel,
    /// 钩子配置标签页
    Hooks,
    /// 网页搜索配置标签页
    WebSearch,
    /// 网络请求配置标签页
    HttpRequest,
    /// 浏览器配置标签页
    Browser,
}

impl SystemTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            SystemTab::General => "常规设置",
            SystemTab::Themes => "主题",
            SystemTab::Editor => "编辑器",
            SystemTab::Projects => "项目管理",
            SystemTab::Providers => "提供商",
            SystemTab::Models => "模型",
            SystemTab::Cron => "定时任务",
            SystemTab::Agents => "智能体",
            SystemTab::Staff => "员工配置",
            SystemTab::Channels => "通道配置",
            SystemTab::Memory => "记忆配置",
            SystemTab::Runtime => "运行时配置",
            SystemTab::Autonomy => "自治配置",
            SystemTab::Security => "安全配置",
            SystemTab::GatewayClient => "客服端网关",
            SystemTab::Gateway => "服务端网关",
            SystemTab::Observability => "可观测性",
            SystemTab::Proxy => "代理配置",
            SystemTab::Tunnel => "隧道配置",
            SystemTab::Hooks => "钩子配置",
            SystemTab::WebSearch => "网络配置",
            SystemTab::HttpRequest => "网络请求配置",
            SystemTab::Browser => "浏览器配置",
        }
    }
    /// 返回所有系统设置标签页的数组
    ///
    /// # 返回值
    ///
    /// 包含所有 `SystemTab` 变体的固定大小数组，按显示顺序排列。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let all_tabs = SystemTab::all();
    /// for tab in all_tabs.iter() {
    ///     println!("{}", tab);
    /// }
    /// ```
    pub(super) fn all() -> [SystemTab; 15] {
        [
            SystemTab::General,
            SystemTab::Providers,
            SystemTab::Models,
            SystemTab::Agents,
            SystemTab::Staff,
            SystemTab::Cron,
            SystemTab::Memory,
            SystemTab::Projects,
            SystemTab::GatewayClient,
            SystemTab::Gateway,
            SystemTab::WebSearch,
            SystemTab::Security,
            SystemTab::Autonomy,
            SystemTab::Hooks,
            SystemTab::Observability,
        ]
    }
}

pub(super) fn is_network_tab(tab: SystemTab) -> bool {
    matches!(
        tab,
        SystemTab::WebSearch
            | SystemTab::HttpRequest
            | SystemTab::Browser
            | SystemTab::Proxy
            | SystemTab::Tunnel
    )
}

impl std::fmt::Display for SystemTab {
    /// 格式化标签页为中文名称
    ///
    /// 将每个标签页变体转换为对应的中文显示名称，
    /// 用于在 UI 界面中渲染标签页标题。
    ///
    /// # 参数
    ///
    /// - `f` - 格式化器引用
    ///
    /// # 返回值
    ///
    /// 返回 `fmt::Result`，表示格式化操作的结果。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

pub(super) fn system_tab_keywords(tab: SystemTab) -> &'static [&'static str] {
    match tab {
        SystemTab::General => &["general", "app", "basic", "基础", "themes", "主题"],
        SystemTab::Themes => {
            &["themes", "gallery", "palette", "custom", "builtin", "画廊", "配色", "色板"]
        }
        SystemTab::Editor => &["editor", "code", "编辑", "主题与显示", "字体大小", "行高"],
        SystemTab::Projects => &["projects", "workspace", "工作区", "Worktree", "历史项目"],
        SystemTab::Providers => {
            &["providers", "api key", "model", "模型", "密钥", "base url", "catalog"]
        }
        SystemTab::Models => &["models", "llm", "ai", "provider"],
        SystemTab::Cron => &["cron", "schedule", "task", "自动化"],
        SystemTab::Agents => &["agents", "subagent", "智能体"],
        SystemTab::Staff => &["staff", "员工"],
        SystemTab::Channels => &["channels", "message", "通道"],
        SystemTab::Memory => &["memory", "记忆"],
        SystemTab::Runtime => &["runtime", "native", "docker", "wasm", "运行时"],
        SystemTab::Autonomy => &["autonomy", "agent run", "自治"],
        SystemTab::Security => &["security", "policy", "安全"],
        SystemTab::GatewayClient => &["gateway client", "connection", "客户端"],
        SystemTab::Gateway => &["gateway server", "api", "服务端"],
        SystemTab::Observability => &["observability", "log", "monitor", "OTel", "trace"],
        SystemTab::Proxy => &["proxy", "代理", "HTTP", "HTTPS", "ALL_PROXY", "NO_PROXY"],
        SystemTab::Tunnel => &["tunnel", "public gateway", "tailscale", "funnel", "tailnet"],
        SystemTab::Hooks => &["hooks", "tool", "command", "钩子"],
        SystemTab::WebSearch => &["network", "网络"],
        SystemTab::HttpRequest => &["http request", "network request"],
        SystemTab::Browser => &["browser", "computer use"],
    }
}

#[cfg(test)]
pub(super) fn system_tab_matches_query(tab: SystemTab, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let needle = query.to_lowercase();
    if tab.to_string().to_lowercase().contains(&needle)
        || system_tab_keywords(tab).iter().any(|text| text.to_lowercase().contains(&needle))
    {
        return true;
    }

    use crate::app::components::system_settings_search::SettingsTabDescriptor;
    fn descriptors_match<T>(descriptors: &[SettingsTabDescriptor<T>], needle: &str) -> bool {
        descriptors.iter().any(|descriptor| {
            descriptor.label.to_lowercase().contains(needle)
                || descriptor.keywords.iter().any(|text| text.to_lowercase().contains(needle))
        })
    }

    match tab {
        SystemTab::General => {
            descriptors_match(super::system_settings_general::GENERAL_TABS, &needle)
        }
        SystemTab::Models => descriptors_match(super::system_settings_models::MODEL_TABS, &needle),
        SystemTab::Cron => descriptors_match(super::system_settings_cron::CRON_TABS, &needle),
        SystemTab::Agents => {
            descriptors_match(super::system_settings_agents::AGENT_CONFIG_TABS, &needle)
                || descriptors_match(
                    &super::system_settings_channels::channel_tab_descriptors(),
                    &needle,
                )
                || descriptors_match(super::system_settings_mcp::MCP_TABS, &needle)
                || descriptors_match(super::system_settings_skills::SKILLS_TABS, &needle)
        }
        SystemTab::Staff => descriptors_match(super::system_settings_staff::search_tabs(), &needle),
        SystemTab::Channels => {
            descriptors_match(&super::system_settings_channels::channel_tab_descriptors(), &needle)
        }
        SystemTab::Memory => {
            descriptors_match(super::system_settings_memory::MEMORY_TABS, &needle)
                || descriptors_match(super::system_settings_memory::QDRANT_TAB, &needle)
        }
        SystemTab::Autonomy => {
            descriptors_match(super::system_settings_autonomy::AUTONOMY_TABS, &needle)
        }
        SystemTab::Security => {
            descriptors_match(super::system_settings_security::SECURITY_TABS, &needle)
        }
        SystemTab::Gateway => {
            descriptors_match(super::system_settings_gateway::GATEWAY_TABS, &needle)
        }
        SystemTab::WebSearch => {
            descriptors_match(super::system_settings_network::NETWORK_TABS, &needle)
        }
        SystemTab::HttpRequest | SystemTab::Browser | SystemTab::Proxy | SystemTab::Tunnel => {
            super::system_settings_network::NETWORK_TABS
                .iter()
                .find(|descriptor| descriptor.tab == tab)
                .is_some_and(|descriptor| {
                    descriptors_match(std::slice::from_ref(descriptor), &needle)
                })
        }
        _ => false,
    }
}

fn search_target_parent_tab(target: SystemSettingsSearchTarget) -> SystemTab {
    match target {
        SystemSettingsSearchTarget::TopLevel(tab) => tab,
        SystemSettingsSearchTarget::General(_) => SystemTab::General,
        SystemSettingsSearchTarget::Models(_) => SystemTab::Models,
        SystemSettingsSearchTarget::Cron(_) => SystemTab::Cron,
        SystemSettingsSearchTarget::Agents(_)
        | SystemSettingsSearchTarget::AgentMcp(_)
        | SystemSettingsSearchTarget::AgentSkills(_) => SystemTab::Agents,
        SystemSettingsSearchTarget::Staff(_) => SystemTab::Staff,
        SystemSettingsSearchTarget::Channels(_) => SystemTab::Agents,
        SystemSettingsSearchTarget::Memory(_) => SystemTab::Memory,
        SystemSettingsSearchTarget::Autonomy(_) => SystemTab::Autonomy,
        SystemSettingsSearchTarget::Security(_) => SystemTab::Security,
        SystemSettingsSearchTarget::Gateway(_) => SystemTab::Gateway,
        SystemSettingsSearchTarget::Network(_) => SystemTab::WebSearch,
    }
}

pub(super) fn search_results(
    app: &App,
    query: &str,
) -> Vec<SystemSettingsSearchEntry> {
    let query = query.trim();
    let mut results = crate::app::components::system_settings_search::catalog(app)
        .into_iter()
        .filter(|entry| {
            if query.is_empty() {
                matches!(entry.target, SystemSettingsSearchTarget::TopLevel(_))
            } else {
                entry.matches(query)
            }
        })
        .collect::<Vec<_>>();

    results.sort_by_key(|entry| {
        let parent = search_target_parent_tab(entry.target);
        SystemTab::all().iter().position(|tab| *tab == parent).unwrap_or(usize::MAX)
    });
    results
}

fn search_target_is_active(app: &App, target: SystemSettingsSearchTarget) -> bool {
    use crate::app::components::system_settings_search::StaffSettingsSearchTab;

    match target {
        SystemSettingsSearchTarget::TopLevel(tab) => {
            tab == app.system_settings_tab
                || (tab == SystemTab::WebSearch && is_network_tab(app.system_settings_tab))
        }
        SystemSettingsSearchTarget::General(tab) => {
            app.system_settings_tab == SystemTab::General && app.general_settings_tab == tab
        }
        SystemSettingsSearchTarget::Models(tab) => {
            app.system_settings_tab == SystemTab::Models && app.model_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Cron(tab) => {
            app.system_settings_tab == SystemTab::Cron && app.cron_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Agents(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab == tab
        }
        SystemSettingsSearchTarget::Staff(tab) => {
            let key = match tab {
                StaffSettingsSearchTab::Roster => "roster",
                StaffSettingsSearchTab::Capabilities => "capabilities",
                StaffSettingsSearchTab::Teams => "teams",
                StaffSettingsSearchTab::Workers => "workers",
            };
            app.system_settings_tab == SystemTab::Staff
                && app.staff_settings.active_tab.key() == key
        }
        SystemSettingsSearchTarget::Channels(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Channels
                && app.channels_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Memory(tab) => {
            app.system_settings_tab == SystemTab::Memory && app.memory_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Autonomy(tab) => {
            app.system_settings_tab == SystemTab::Autonomy && app.autonomy_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Security(tab) => {
            app.system_settings_tab == SystemTab::Security && app.security_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Gateway(tab) => {
            app.system_settings_tab == SystemTab::Gateway && app.gateway_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::Network(tab) => app.system_settings_tab == tab,
        SystemSettingsSearchTarget::AgentMcp(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Mcp
                && app.mcp_settings.active_tab == tab
        }
        SystemSettingsSearchTarget::AgentSkills(tab) => {
            app.system_settings_tab == SystemTab::Agents
                && app.agents_settings.active_config_tab
                    == crate::app::state::AgentConfigTab::Skills
                && app.skills_settings.active_tab == tab
        }
    }
}

pub(super) fn active_tab_help_modal_open(app: &App, active_tab: SystemTab) -> bool {
    let help_tab = active_settings_help_tab(app, active_tab);
    crate::app::components::system_settings_help::help_open_for_tab(
        app.system_settings_help_tab,
        help_tab,
    ) || match active_tab {
        SystemTab::Agents => match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Heartbeat => app.heartbeat_settings.show_help_modal,
            crate::app::state::AgentConfigTab::Skills => app.skills_settings.show_help_modal,
            crate::app::state::AgentConfigTab::Reliability => {
                app.reliability_settings.show_help_modal
            }
            _ => false,
        },
        SystemTab::Cron => {
            app.cron_settings.show_help_modal || app.scheduler_settings.show_help_modal
        }
        SystemTab::Autonomy => app.autonomy_settings.show_help_modal,
        SystemTab::Security => app.security_settings.show_help_modal,
        SystemTab::GatewayClient => app.gateway_client_settings.show_help_modal,
        SystemTab::Gateway => app.gateway_settings.show_help_modal,
        SystemTab::Observability => app.observability_settings.show_help_modal,
        SystemTab::Proxy => app.proxy_settings.show_help_modal,
        _ => false,
    }
}

fn active_settings_help_tab(app: &App, active_tab: SystemTab) -> SystemTab {
    if active_tab != SystemTab::Agents {
        return active_tab;
    }

    match app.agents_settings.active_config_tab {
        crate::app::state::AgentConfigTab::Channels => SystemTab::Channels,
        crate::app::state::AgentConfigTab::Runtime => SystemTab::Runtime,
        _ => active_tab,
    }
}

pub(super) fn active_tab_save_error(app: &App, active_tab: SystemTab) -> Option<&str> {
    match active_tab {
        SystemTab::Providers => app.provider_settings.save_error.as_deref(),
        SystemTab::Models => {
            crate::app::components::system_settings_models::active_tab_save_error(app)
        }
        SystemTab::Cron => {
            app.cron_settings.save_error.as_deref().or(app.scheduler_settings.save_error.as_deref())
        }
        SystemTab::Agents => match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Agents => app.agents_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Channels => {
                app.channels_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Runtime => app.runtime_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Heartbeat => {
                app.heartbeat_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Acp => app.acp_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Mcp => app.mcp_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Skills => app.skills_settings.save_error.as_deref(),
            crate::app::state::AgentConfigTab::Plugins => None,
            crate::app::state::AgentConfigTab::Reliability => {
                app.reliability_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::DialogueFlow
            | crate::app::state::AgentConfigTab::QuickMessages => None,
        },
        SystemTab::Staff => app.staff_settings.save_error.as_deref(),
        SystemTab::Channels => app.channels_settings.save_error.as_deref(),
        SystemTab::Memory => match app.memory_settings.active_tab {
            crate::app::state::MemorySettingsTab::Storage => {
                app.storage_settings.save_error.as_deref()
            }
            _ => app.memory_settings.save_error.as_deref(),
        },
        SystemTab::Runtime => app.runtime_settings.save_error.as_deref(),
        SystemTab::Autonomy => app.autonomy_settings.save_error.as_deref(),
        SystemTab::Security => app.security_settings.save_error.as_deref(),
        SystemTab::GatewayClient => app.gateway_client_settings.save_error.as_deref(),
        SystemTab::Gateway => app.gateway_settings.save_error.as_deref(),
        SystemTab::Observability => app.observability_settings.save_error.as_deref(),
        SystemTab::Proxy => app.proxy_settings.save_error.as_deref(),
        SystemTab::Tunnel => app.tunnel_settings.save_error.as_deref(),
        SystemTab::Hooks => app.hooks_settings.save_error.as_deref(),
        SystemTab::WebSearch => app.web_search_settings.save_error.as_deref(),
        SystemTab::HttpRequest => app.http_request_settings.save_error.as_deref(),
        SystemTab::Browser => app.browser_settings.save_error.as_deref(),
        SystemTab::Themes => app.themes_settings.error.as_deref(),
        SystemTab::General => match app.general_settings_tab {
            crate::app::state::GeneralSettingsTab::Themes => app.themes_settings.error.as_deref(),
            _ => None,
        },
        SystemTab::Editor | SystemTab::Projects => None,
    }
}

pub(super) fn tabbed_page_owns_save_error(active_tab: SystemTab) -> bool {
    matches!(
        active_tab,
        SystemTab::Models
            | SystemTab::Cron
            | SystemTab::Agents
            | SystemTab::Staff
            | SystemTab::Channels
            | SystemTab::Memory
            | SystemTab::Autonomy
            | SystemTab::Security
            | SystemTab::Gateway
            | SystemTab::WebSearch
            | SystemTab::HttpRequest
            | SystemTab::Browser
            | SystemTab::Proxy
            | SystemTab::Tunnel
    )
}

/// 渲染系统设置面板视图
///
/// 构建并返回系统设置模态对话框的 UI 元素。该视图包含：
///
/// - 半透明遮罩层（点击可关闭设置面板）
/// - 模态对话框容器
/// - 左侧标签页导航栏
/// - 右侧配置内容区域
/// - 关闭按钮
///
/// # 参数
///
/// - `app` - 应用状态引用，包含当前激活的标签页和显示状态等信息
///
/// # 返回值
///
/// 返回 `Element<'_, Message>` 类型的 UI 元素，可直接集成到 Iced 应用中。
///
/// # 行为说明
///
/// 1. 如果 `app.show_system_settings` 为 `false`，返回空容器
/// 2. 根据当前激活标签页加载对应的配置模块视图
/// 3. 对于 Providers 和 Models 标签页，额外添加浮层支持
/// 4. 点击遮罩层时触发关闭设置面板的消息
///
/// # 布局结构
///
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ 半透明遮罩 (点击关闭)                              │
/// │  ┌───────────────────────────────────────────┐  │
/// │  │ 系统配置                          [×]     │  │
/// │  ├─────────────┬─────────────────────────────┤  │
/// │  │ 标签页列表   │ 配置内容区域                 │  │
/// │  │ · 常规设置  │                             │  │
/// │  │ · 对话流    │ (根据选中标签页显示)          │  │
/// │  │ · 编辑器    │                             │  │
/// │  │ · ...      │                             │  │
/// │  │            │                             │  │
/// │  └─────────────┴─────────────────────────────┘  │
/// └─────────────────────────────────────────────────┘
/// ```
///
/// # 示例
///
/// ```ignore
/// let settings_view = view(&app);
/// // 将 settings_view 添加到应用的视图层级中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 如果设置面板未显示，返回空容器
    if !app.show_system_settings {
        return container(column![]).into();
    }

    // 获取当前激活的标签页
    let active_tab = app.system_settings_tab;
    let help_tab = active_settings_help_tab(app, active_tab);
    let filtered_tabs = search_results(app, &app.system_settings_query);
    let query_input = text_input("搜索配置", &app.system_settings_query)
        .on_input(|value| Message::Settings(message::SettingsMessage::SystemTabQueryChanged(value)))
        .padding([8, 10])
        .size(13)
        .style(settings_text_input_style);

    // 构建左侧标签页导航栏
    // 为每个标签页创建按钮，根据激活状态和悬停状态应用不同样式
    let tabs_bar: Element<'_, Message> = if filtered_tabs.is_empty() {
        container(text("没有匹配的配置").size(12).style(settings_muted_text_style))
            .padding([10, 12])
            .width(Length::Fill)
            .into()
    } else {
        column(
            filtered_tabs
                .iter()
                .map(|entry| {
                    // 判断当前标签页是否处于激活状态
                    let is_active = search_target_is_active(app, entry.target);
                    // 创建标签页文本标签
                    let label: Element<'_, Message> = if let Some(parent) = entry.parent_label {
                        column![
                            text(entry.label).size(13),
                            text(parent).size(11).style(settings_muted_text_style),
                        ]
                        .spacing(2)
                        .into()
                    } else {
                        text(entry.label).size(13).into()
                    };

                    // 构建标签页按钮
                    let btn = button(container(label).width(Length::Fill).padding([8, 12]))
                        .width(Length::Fill)
                        .on_press(Message::Settings(
                            message::SettingsMessage::SystemSearchTargetSelected(entry.target),
                        ))
                        .style(move |theme: &iced::Theme, status| {
                            let palette = theme.extended_palette();
                            let is_dark = theme.palette().background.r
                                + theme.palette().background.g
                                + theme.palette().background.b
                                < 1.5;
                            let bg = if is_active {
                                Some(Background::Color(if is_dark {
                                    theme.palette().primary.scale_alpha(0.18)
                                } else {
                                    theme.palette().primary.scale_alpha(0.08)
                                }))
                            } else {
                                match status {
                                    iced::widget::button::Status::Hovered => {
                                        Some(Background::Color(if is_dark {
                                            palette.background.weak.color.scale_alpha(0.72)
                                        } else {
                                            Color::WHITE.scale_alpha(0.78)
                                        }))
                                    }
                                    iced::widget::button::Status::Pressed => {
                                        Some(Background::Color(if is_dark {
                                            palette.background.strong.color.scale_alpha(0.86)
                                        } else {
                                            palette.background.weak.color.scale_alpha(0.92)
                                        }))
                                    }
                                    _ => None,
                                }
                            };

                            iced::widget::button::Style {
                                background: bg,
                                text_color: if is_active {
                                    theme.palette().primary.scale_alpha(0.96)
                                } else {
                                    theme.palette().text.scale_alpha(0.92)
                                },
                                border: Border {
                                    radius: 14.0.into(),
                                    width: 1.0,
                                    color: if is_active {
                                        theme.palette().primary.scale_alpha(0.24)
                                    } else if is_dark {
                                        palette.background.strong.color.scale_alpha(0.78)
                                    } else {
                                        Color::from_rgba8(15, 23, 42, 0.06)
                                    },
                                },
                                shadow: if is_active {
                                    Shadow {
                                        color: Color::BLACK.scale_alpha(if is_dark {
                                            0.18
                                        } else {
                                            0.06
                                        }),
                                        offset: Vector::new(0.0, 6.0),
                                        blur_radius: 14.0,
                                    }
                                } else {
                                    Shadow::default()
                                },
                                ..Default::default()
                            }
                        });

                    container(btn).width(Length::Fill).into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(4)
        .padding([2, 2])
        .into()
    };

    // 根据当前激活的标签页加载对应的配置内容视图
    let content: Element<'_, Message> = match active_tab {
        SystemTab::General => crate::app::components::system_settings_general::view(app),
        SystemTab::Themes => crate::app::components::system_settings_themes::view(app),
        SystemTab::Editor => crate::app::components::system_settings_editor::view(app),
        SystemTab::Projects => crate::app::components::system_settings_projects::view(app),
        SystemTab::Providers => {
            crate::app::components::system_settings_providers::connected::view(app)
        }
        SystemTab::Models => crate::app::components::system_settings_models::view(app),
        SystemTab::Cron => crate::app::components::system_settings_cron::view(app),
        SystemTab::Agents => crate::app::components::system_settings_agents::view(app),
        SystemTab::Staff => crate::app::components::system_settings_staff::view(app),
        SystemTab::Channels => crate::app::components::system_settings_channels::view(app),
        SystemTab::Memory => crate::app::components::system_settings_memory::view(app),
        SystemTab::Runtime => crate::app::components::system_settings_runtime::view(app),
        SystemTab::Autonomy => crate::app::components::system_settings_autonomy::view(app),
        SystemTab::Security => crate::app::components::system_settings_security::view(app),
        SystemTab::GatewayClient => {
            crate::app::components::system_settings_gateway_client::view(app)
        }
        SystemTab::Gateway => crate::app::components::system_settings_gateway::view(app),
        SystemTab::Observability => {
            crate::app::components::system_settings_observability::view(app)
        }
        SystemTab::Hooks => crate::app::components::system_settings_hooks::view(app),
        SystemTab::WebSearch
        | SystemTab::HttpRequest
        | SystemTab::Browser
        | SystemTab::Proxy
        | SystemTab::Tunnel => {
            crate::app::components::system_settings_network::view(app)
        }
    };

    // 左侧返回按钮和右侧关闭按钮执行相同的关闭操作
    let back_btn = icon_btn(
        Icon::ArrowLeft,
        "返回",
        Some(Message::View(message::ViewMessage::ToggleSystemSettings)),
    );
    let close_btn =
        settings_close_button(Message::View(message::ViewMessage::ToggleSystemSettings));

    // 构建全屏设置面板主体内容
    // 面板占满内容区，不再以小尺寸模态框居中显示
    let modal_content = container(
        column![
            container(
                row![
                    column![
                        row![text("配置").size(22), back_btn].spacing(8).align_y(Alignment::Center),
                        text("管理运行、工具、模型与连接设置。")
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4),
                    container(text(" ")).width(Length::Fill),
                    close_btn,
                ]
                .align_y(Alignment::Start)
            )
            .padding(iced::Padding::default().top(18).right(18).bottom(14).left(18)),
            row![
                container(
                    column![
                        text("分类").size(11).style(settings_muted_text_style),
                        query_input,
                        scrollable(
                            container(tabs_bar).padding(iced::Padding::default().right(10.0))
                        )
                        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                        .height(Length::Fill)
                    ]
                    .spacing(8)
                    .height(Length::Fill),
                )
                .width(Length::Fixed(272.0))
                .height(Length::Fill)
                .padding([16, 14])
                .style(|theme: &iced::Theme| {
                    let extended = theme.extended_palette();
                    let is_dark = theme.palette().background.r
                        + theme.palette().background.g
                        + theme.palette().background.b
                        < 1.5;
                    iced::widget::container::Style {
                        background: Some(Background::Color(if is_dark {
                            extended.background.base.color.scale_alpha(0.66)
                        } else {
                            Color::WHITE.scale_alpha(0.72)
                        })),
                        border: Border {
                            radius: 20.0.into(),
                            width: 1.0,
                            color: if is_dark {
                                extended.background.strong.color.scale_alpha(0.82)
                            } else {
                                Color::from_rgba8(15, 23, 42, 0.08)
                            },
                        },
                        shadow: Shadow {
                            color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
                            offset: Vector::new(0.0, 10.0),
                            blur_radius: 24.0,
                        },
                        snap: false,
                        ..Default::default()
                    }
                }),
                container({
                    let mut content_with_help = column![].spacing(12).width(Length::Fill);
                    if let Some(help_bar) =
                        crate::app::components::system_settings_help::help_button_bar(help_tab)
                    {
                        content_with_help = content_with_help.push(help_bar);
                    }
                    if !tabbed_page_owns_save_error(active_tab)
                        && let Some(err) = active_tab_save_error(app, active_tab)
                    {
                        content_with_help = content_with_help.push(settings_error_banner(err));
                    }
                    content_with_help = content_with_help.push(content);

                    let content_panel: Element<'_, Message> = if active_tab == SystemTab::Agents
                        || active_tab == SystemTab::Staff
                        || active_tab_help_modal_open(app, active_tab)
                    {
                        container(content_with_help)
                            .padding([0, 0])
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    } else {
                        scrollable(
                            container(content_with_help)
                                .padding(iced::Padding::default().right(10.0)),
                        )
                        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                        .into()
                    };
                    container(content_panel)
                        .padding([18, 20])
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(settings_panel_style)
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::default().top(0).right(20).bottom(20).left(0))
            ]
            .spacing(16)
            .height(Length::Fill)
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|theme: &iced::Theme| {
        let palette = theme.extended_palette();
        let is_dark = theme.palette().background.r
            + theme.palette().background.g
            + theme.palette().background.b
            < 1.5;
        iced::widget::container::Style {
            text_color: Some(theme.palette().text),
            background: Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.28)
            } else {
                Color::from_rgba8(248, 250, 252, 0.96)
            })),
            border: Border::default(),
            snap: false,
            ..Default::default()
        }
    });

    // 将模态内容转换为 Element
    let mut dialog: Element<'_, Message> = modal_content.into();

    // 为 Providers 标签页添加额外的浮层视图
    // 这些浮层用于显示自定义提供商、目录、连接等子对话框
    if app.system_settings_tab == SystemTab::Providers {
        dialog = crate::app::components::system_settings_providers::custom_provider::view_overlays(
            app, dialog,
        );
        dialog =
            crate::app::components::system_settings_providers::catalog::view_overlays(app, dialog);
        dialog =
            crate::app::components::system_settings_providers::connect::view_overlays(app, dialog);
        dialog =
            crate::app::components::system_settings_providers::custom_model_modal::view_overlays(
                app, dialog,
            );
    }

    // 为 Models 标签页添加额外的浮层视图
    if app.system_settings_tab == SystemTab::Models {
        dialog = crate::app::components::system_settings_models::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Agents {
        dialog = match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Heartbeat => {
                crate::app::components::system_settings_heartbeat::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Mcp => {
                crate::app::components::system_settings_mcp::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Skills => {
                crate::app::components::system_settings_skills::view_overlays(app, dialog)
            }
            crate::app::state::AgentConfigTab::Reliability => {
                crate::app::components::system_settings_reliability::view_overlays(app, dialog)
            }
            _ => dialog,
        };
    }

    if app.system_settings_tab == SystemTab::Cron {
        dialog = crate::app::components::system_settings_cron::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Autonomy {
        dialog = crate::app::components::system_settings_autonomy::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Security {
        dialog = crate::app::components::system_settings_security::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::GatewayClient {
        dialog = crate::app::components::system_settings_gateway_client::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Gateway {
        dialog = crate::app::components::system_settings_gateway::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Observability {
        dialog = crate::app::components::system_settings_observability::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Proxy {
        dialog = crate::app::components::system_settings_proxy::view_overlays(app, dialog);
    }

    dialog = crate::app::components::system_settings_help::with_help_modal(
        app,
        dialog,
        help_tab,
        app.system_settings_help_tab,
    );

    dialog
}
