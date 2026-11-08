//! 系统设置外壳、分类导航和当前设置页的分派。
//!
//! 分类列表和搜索结果共用跳转目标，具体配置及浮层状态由各设置页维护。

use crate::app::components::system_settings_common::{
    settings_close_button, settings_error_banner,
};
use crate::app::components::system_settings_help;
use crate::app::state::{GeneralSettingsTab, ThemesSettingsMode};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, container, responsive, row, rule, scrollable};
use iced::{Alignment, Element, Length, Theme};

mod field_scroll;
pub(super) mod general_fields;
mod navigation;

pub(crate) use field_scroll::reveal_field;

pub(crate) use navigation::SETTINGS_NAVIGATION_SCROLL;

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
    /// 应用目录与安装诊断
    Apps,
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
            SystemTab::Apps => "应用列表",
            SystemTab::General => "常规设置",
            SystemTab::Themes => "主题",
            SystemTab::Editor => "编辑器",
            SystemTab::Projects => "项目管理",
            SystemTab::Providers => "提供商",
            SystemTab::Models => "模型",
            SystemTab::Cron => "定时任务",
            SystemTab::Agents => "智能体",
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
    /// 按侧边栏分组顺序列出一级设置页。
    pub(super) fn all() -> impl Iterator<Item = SystemTab> {
        navigation::GROUPS.iter().flat_map(|(_, tabs)| tabs.iter().copied())
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
        SystemTab::Apps => &["apps", "应用", "安装", "catalog"],
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
        SystemTab::Channels => &["channels", "message", "通道"],
        SystemTab::Memory => &["memory", "记忆"],
        SystemTab::Runtime => &["runtime", "native", "docker", "wasm", "运行时"],
        SystemTab::Autonomy => &["autonomy", "agent run", "自治"],
        SystemTab::Security => &["security", "policy", "安全"],
        SystemTab::GatewayClient => {
            &["gateway client", "connection", "客户端", "我的图标", "图标库"]
        }
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
            crate::app::state::AgentConfigTab::Runtime => {
                app.runtime_settings.save_error.as_deref()
            }
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
        SystemTab::Apps
        | SystemTab::Editor
        | SystemTab::General
        | SystemTab::Projects
        | SystemTab::Themes => None,
    }
}

pub(super) fn tabbed_page_owns_save_error(active_tab: SystemTab) -> bool {
    matches!(
        active_tab,
        SystemTab::Models
            | SystemTab::Cron
            | SystemTab::Agents
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

const SETTINGS_CONTENT_MAX_WIDTH: f32 = 860.0;
const SETTINGS_SCROLL_GUTTER: u16 = 7;
pub(crate) const SETTINGS_CONTENT_SCROLL: &str = "system-settings-content-scroll";

fn settings_content(app: &App, active_tab: SystemTab) -> Element<'_, Message> {
    let content: Element<'_, Message> = match active_tab {
        SystemTab::Apps => super::system_settings_apps::view(&app.apps_catalog),
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
        SystemTab::Channels => crate::app::components::system_settings_channels::view(app),
        SystemTab::Memory => crate::app::components::system_settings_memory::view(app),
        SystemTab::Runtime => crate::app::components::system_settings_runtime::view(app),
        SystemTab::Autonomy => crate::app::components::system_settings_autonomy::view(app),
        SystemTab::Security => crate::app::components::system_settings_security::view(app),
        SystemTab::GatewayClient => column![
            super::system_settings_agent_apps::view(&app.agent_apps),
            crate::app::components::system_settings_gateway_client::view(app),
        ]
        .spacing(16)
        .into(),
        SystemTab::Gateway => crate::app::components::system_settings_gateway::view(app),
        SystemTab::Observability => {
            crate::app::components::system_settings_observability::view(app)
        }
        SystemTab::Hooks => crate::app::components::system_settings_hooks::view(app),
        SystemTab::WebSearch
        | SystemTab::HttpRequest
        | SystemTab::Browser
        | SystemTab::Proxy
        | SystemTab::Tunnel => crate::app::components::system_settings_network::view(app),
    };

    let mut content_column = column![].spacing(12).width(Length::Fill);
    if !tabbed_page_owns_save_error(active_tab)
        && let Some(err) = active_tab_save_error(app, active_tab)
    {
        let localized = match active_tab {
            SystemTab::Providers => app.provider_settings.save_error.as_ref(),
            SystemTab::GatewayClient => app.gateway_client_settings.save_error.as_ref(),
            SystemTab::Gateway => app.gateway_settings.save_error.as_ref(),
            _ => None,
        }
        .map(|error| error.display(app.desktop_locale))
        .unwrap_or_else(|| {
            if active_tab == SystemTab::Runtime
                || (active_tab == SystemTab::Agents
                    && app.agents_settings.active_config_tab
                        == crate::app::state::AgentConfigTab::Runtime)
            {
                crate::app::i18n::settings_runtime::validation_error(app.desktop_locale, err)
            } else {
                err.to_owned()
            }
        });
        content_column = content_column.push(settings_error_banner(localized));
    }
    let content_column = content_column.push(content);
    let page: Element<'_, Message> =
        if active_tab == SystemTab::Agents || active_tab_help_modal_open(app, active_tab) {
            centered_content(content_column.height(Length::Fill)).height(Length::Fill).into()
        } else {
            scrollable(centered_content(content_column))
                .id(SETTINGS_CONTENT_SCROLL)
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill)
                .into()
        };

    container(page).width(Length::Fill).height(Length::Fill).clip(true).into()
}

fn centered_content<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(container(content).width(Length::Fill).max_width(SETTINGS_CONTENT_MAX_WIDTH))
        .padding([0, SETTINGS_SCROLL_GUTTER])
        .center_x(Length::Fill)
}

fn settings_shell(
    app: &App,
    width: f32,
    active_tab: SystemTab,
    help_tab: SystemTab,
) -> Element<'_, Message> {
    let close_message = Message::View(message::ViewMessage::ToggleSystemSettings);
    let mut actions = row![].spacing(6).align_y(Alignment::Center);
    if let Some(help) = system_settings_help::help_button(app.desktop_locale, help_tab) {
        actions = actions.push(help);
    }
    let actions = container(actions.push(settings_close_button(close_message)))
        .align_right(Length::Fill)
        .padding([8, 12]);

    let sidebar_width = (width * 0.24).clamp(112.0, 300.0);
    let divider = rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: theme.palette().text.scale_alpha(0.12),
        ..rule::default(theme)
    });
    let content = column![actions, settings_content(app, active_tab)]
        .width(Length::Fill)
        .height(Length::Fill);
    row![navigation::view(app, sidebar_width), divider, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 渲染全屏设置、分类导航及当前分类的浮层。
pub fn view(app: &App) -> Element<'_, Message> {
    if !app.show_system_settings {
        return container(column![]).into();
    }

    let active_tab = app.system_settings_tab;
    let help_tab = active_settings_help_tab(app, active_tab);
    let shell = responsive(move |size| settings_shell(app, size.width, active_tab, help_tab));
    let modal_content =
        container(shell).width(Length::Fill).height(Length::Fill).style(|theme: &Theme| {
            container::Style {
                text_color: Some(theme.palette().text),
                background: Some(theme.palette().background.into()),
                ..Default::default()
            }
        });

    let themes_active = active_tab == SystemTab::Themes
        || (active_tab == SystemTab::General
            && app.general_settings_tab == GeneralSettingsTab::Themes);
    let editing_theme = themes_active && app.themes_settings.mode == ThemesSettingsMode::Edit;
    let select_focus_available = app.system_settings_help_tab.is_none()
        && (active_tab == SystemTab::Editor
            || (active_tab == SystemTab::Autonomy && !app.autonomy_settings.show_help_modal)
            || (active_tab == SystemTab::Security && !app.security_settings.show_help_modal)
            || (active_tab == SystemTab::Observability
                && !app.observability_settings.show_help_modal)
            || (active_tab == SystemTab::General
                && matches!(
                    app.general_settings_tab,
                    GeneralSettingsTab::Interface
                        | GeneralSettingsTab::Terminal
                        | GeneralSettingsTab::Editor
                )));

    let mut dialog: Element<'_, Message> = if select_focus_available {
        modal_content.id(SELECT_FOCUS_SCOPE).into()
    } else {
        modal_content.into()
    };

    if editing_theme {
        dialog = super::system_settings_themes::editor_overlay(app, dialog);
        dialog = container(dialog).id(SELECT_FOCUS_SCOPE).into();
    }

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

    if app.system_settings_tab == SystemTab::Apps {
        dialog = super::system_settings_apps::view_overlays(
            &app.apps_catalog,
            crate::app::i18n::I18n::new(app.desktop_locale),
            &app.app_theme,
            dialog,
        );
        dialog = super::app_providers::list::overlay(
            &app.app_providers,
            super::app_providers::list::Help::new(
                crate::app::i18n::I18n::new(app.desktop_locale),
                &app.app_theme,
            ),
            dialog,
        );
    }
    if app.system_settings_tab == SystemTab::GatewayClient {
        dialog = crate::app::components::system_settings_gateway_client::view_overlays(app, dialog);
        dialog = super::system_settings_agent_apps::view_overlays(&app.agent_apps, dialog);
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

pub(super) const SELECT_FOCUS_SCOPE: &str = "system-settings-select-focus";

pub(crate) fn traverse_select_focus(reverse: bool) -> iced::Task<Message> {
    use iced::advanced::widget::operation::{focusable, scope};
    use iced::advanced::widget::{Id, operate};

    let scope_id = Id::from(SELECT_FOCUS_SCOPE);
    if reverse {
        operate(scope(scope_id, focusable::focus_previous::<()>())).discard()
    } else {
        operate(scope(scope_id, focusable::focus_next::<()>())).discard()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "system_settings_ui_refresh_04_tests.rs"]
mod ui_refresh_04_tests;
