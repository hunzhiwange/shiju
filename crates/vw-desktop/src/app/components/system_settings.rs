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
//! - **可靠性配置 (Reliability)**：系统可靠性配置
//! - **通道配置 (Channels)**：CLI 与多通道消息集成配置
//! - **记忆配置 (Memory)**：记忆后端、缓存和嵌入检索配置
//! - **运行时配置 (Runtime)**：native/docker/wasm 运行时配置
//! - **ACP 配置 (Acp)**：ACP 智能体目录、初始化说明与启用状态
//! - **MCP 配置 (Mcp)**：全局 MCP 服务器配置
//! - **自治配置 (Autonomy)**：代理自治能力设置
//! - **安全配置 (Security)**：安全策略配置
//! - **网关配置 (Gateway)**：HTTP 网关、Auth 鉴权与 node-control 配置
//! - **可观测性配置 (Observability)**：日志与监控配置
//! - **代理配置 (Proxy)**：网络代理设置
//! - **隧道配置 (Tunnel)**：网关公网暴露隧道配置
//! - **技能配置 (Skills)**：代理技能配置
//! - **钩子配置 (Hooks)**：运行时钩子配置
//! - **网络配置 (WebSearch / HttpRequest / Browser)**：网页搜索、HTTP 请求与浏览器工具配置

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    icon_btn, settings_close_button, settings_error_banner, settings_muted_text_style,
    settings_panel_style, settings_text_input_style,
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
    /// 主题管理标签页
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
    /// 可靠性配置标签页
    Reliability,
    /// 通道配置标签页
    Channels,
    /// 记忆配置标签页
    Memory,
    /// 运行时配置标签页
    Runtime,
    /// ACP 配置标签页
    Acp,
    /// MCP 配置标签页
    Mcp,
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
    /// 技能配置标签页
    Skills,
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
    pub(super) fn all() -> [SystemTab; 24] {
        [
            SystemTab::General,
            SystemTab::Themes,
            SystemTab::Projects,
            SystemTab::Providers,
            SystemTab::Models,
            SystemTab::Memory,
            SystemTab::Cron,
            SystemTab::Agents,
            SystemTab::Staff,
            SystemTab::Reliability,
            SystemTab::Channels,
            SystemTab::Runtime,
            SystemTab::Acp,
            SystemTab::Mcp,
            SystemTab::Autonomy,
            SystemTab::Security,
            SystemTab::GatewayClient,
            SystemTab::Gateway,
            SystemTab::Observability,
            SystemTab::Proxy,
            SystemTab::Tunnel,
            SystemTab::Skills,
            SystemTab::Hooks,
            SystemTab::WebSearch,
        ]
    }
}

pub(super) fn is_network_tab(tab: SystemTab) -> bool {
    matches!(tab, SystemTab::WebSearch | SystemTab::HttpRequest | SystemTab::Browser)
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
        match self {
            SystemTab::General => write!(f, "常规设置"),
            SystemTab::Themes => write!(f, "主题"),
            SystemTab::Editor => write!(f, "编辑器"),
            SystemTab::Projects => write!(f, "项目"),
            SystemTab::Providers => write!(f, "提供商"),
            SystemTab::Models => write!(f, "模型"),
            SystemTab::Cron => write!(f, "定时任务配置"),
            SystemTab::Agents => write!(f, "智能体配置"),
            SystemTab::Staff => write!(f, "员工配置"),
            SystemTab::Reliability => write!(f, "可靠性配置"),
            SystemTab::Channels => write!(f, "通道配置"),
            SystemTab::Memory => write!(f, "记忆配置"),
            SystemTab::Runtime => write!(f, "运行时配置"),
            SystemTab::Acp => write!(f, "ACP 配置"),
            SystemTab::Mcp => write!(f, "MCP 配置"),
            SystemTab::Autonomy => write!(f, "自治配置"),
            SystemTab::Security => write!(f, "安全配置"),
            SystemTab::GatewayClient => write!(f, "客户端网关"),
            SystemTab::Gateway => write!(f, "服务端网关"),
            SystemTab::Observability => write!(f, "可观测性配置"),
            SystemTab::Proxy => write!(f, "代理配置"),
            SystemTab::Tunnel => write!(f, "隧道配置"),
            SystemTab::Skills => write!(f, "技能配置"),
            SystemTab::Hooks => write!(f, "钩子配置"),
            SystemTab::WebSearch => write!(f, "网络配置"),
            SystemTab::HttpRequest => write!(f, "网络请求配置"),
            SystemTab::Browser => write!(f, "浏览器配置"),
        }
    }
}

fn system_tab_search_text(tab: SystemTab) -> &'static str {
    match tab {
        SystemTab::General => {
            "常规设置 general app basic 基础 界面 语言 应用主题 自动保存 终端 终端 Shell 终端主题 终端字体 字体大小 编辑器 editor code 编辑器主题 行高 preview auto save"
        }
        SystemTab::Themes => {
            "主题 themes gallery palette custom builtin 画廊 配色 色板 自定义主题 内置主题 应用 删除 新建主题"
        }
        SystemTab::Editor => {
            "编辑器 editor code 编辑 主题与显示 编辑器主题 跟随系统主题 字体大小 行高 自动调整行高"
        }
        SystemTab::Projects => {
            "项目 projects workspace 工作区 当前项目 打开文件夹 Worktree 历史项目 项目名称 打开项目 保存 删除"
        }
        SystemTab::Providers => {
            "提供商 providers api key model 模型 API Key 密钥 连接 自定义提供商 base url endpoint catalog"
        }
        SystemTab::Models => {
            "模型 models llm ai provider 搜索模型 模型详情 提供商 模型 id 上下文窗口 最大输出 温度 模型路由 routing 匹配模式 pattern priority 优先级 查询模型分类 classification rules 规则 分类 多模态配置 multimodal image vision 图片模型 OCR 文字提取 最大图片数量 单张图片大小上限 远程抓取 remote fetch"
        }
        SystemTab::Cron => {
            "定时任务配置 cron schedule task 自动化 启用 历史留存 history retention 调度器 scheduler 执行能力 最大任务 并发上限 最大并发 max_tasks max_concurrent"
        }
        SystemTab::Agents => {
            "智能体配置 agents subagent model tools 模型 工具 Agent 工作台 新增智能体 key 基础 身份 工具 技能 提供商 API 密钥 温度 启用 最大深度 上下文压缩 历史消息上限 并行工具 工具调度器 最大迭代次数 智能体模式 对话流 dialogue flow permission 对话 权限 时间线 显示推理摘要 展开 shell 展开编辑工具 reasoning summary edit patch 快捷消息 quick messages chat 输入框 常用操作 Git 操作 创建新会话 自定义消息 动作 心跳配置 heartbeat monitor follow up 自动化 执行间隔 兜底消息 目标通道 接收者 投递内容"
        }
        SystemTab::Staff => {
            "员工配置 staff roster task board agent binding capability 员工 名册 任务看板 绑定 能力 工具 技能"
        }
        SystemTab::Reliability => {
            "可靠性配置 reliability retry run 运行 Provider 重试次数 退避 频道与调度 频道重连 调度器重试"
        }
        SystemTab::Channels => {
            "通道配置 channels cli message CLI 通道 项目目录 已启用通道 workspace Slack Teams 外部消息"
        }
        SystemTab::Memory => {
            "记忆配置 memory embedding routes vector 模型 记忆存储 storage database 存储后端 连接串 Schema 表名 TLS 持久化 清理 快照 自动恢复 保留 缓存 归档 SQLite 打开超时 嵌入与检索 嵌入模型 嵌入路由 路由规则 匹配模式 提供商 API Key 维度 混合检索 权重"
        }
        SystemTab::Runtime => {
            "运行时配置 runtime native docker wasm run 运行 运行时类型 推理级别 只读根文件系统 挂载工作区 workspace read write symlink host validation"
        }
        SystemTab::Acp => {
            "acp agent client protocol 智能体 后端 启用 禁用 初始化 环境 配置 codex claude gemini copilot pi npx"
        }
        SystemTab::Mcp => {
            "mcp model context protocol mcpServers server tool 工具 服务器 stdio http sse headers env allowedTools deniedTools timeout 插件 plugin"
        }
        SystemTab::Autonomy => {
            "自治配置 autonomy agent run approval 运行 权限与审批 执行级别 审批阈值 重定向 预算 工具与路径 shell 命令 自动审批 非 CLI 暴露 审批人"
        }
        SystemTab::Security => {
            "安全配置 security policy permission 沙箱 后端说明 资源限制 内存 CPU 子进程 审计日志 OTP 紧急停止 E-Stop 系统调用异常检测 Canary Token 语义注入防护"
        }
        SystemTab::GatewayClient => {
            "客户端网关 gateway client connection 用户名 密码 网关地址 skey Auth 连接 heartbeat"
        }
        SystemTab::Gateway => {
            "服务端网关 gateway server api 监听地址 端口 skey auth_enabled Auth 公网绑定 反向代理 webhook 幂等性 node-control"
        }
        SystemTab::Observability => {
            "可观测性配置 observability log monitor 观测后端 OTel 端点 运行时追踪 trace 模式 文件路径 rolling 保留量"
        }
        SystemTab::Proxy => {
            "代理配置 proxy network 生效范围 代理地址 HTTP HTTPS ALL_PROXY NO_PROXY 排除与路由 指定服务 selector"
        }
        SystemTab::Tunnel => {
            "隧道配置 tunnel public gateway 接入方式 提供者 tailscale funnel tailnet 主机名 端口 公网暴露"
        }
        SystemTab::Skills => {
            "技能配置 skills tools plugins 工具 插件 open-skills 目录 模式 catalog browser"
        }
        SystemTab::Hooks => {
            "钩子配置 hooks tool command 工具 基础行为 启用 command_logger 内置钩子 自定义 hooks"
        }
        SystemTab::WebSearch => {
            "网络配置 network 网页搜索 web search brave tool 工具 搜索 基础行为 启用 搜索服务 DuckDuckGo Brave Serper Google Bing Brave 密钥 API 密钥 接口地址 请求参数 结果数量 超时时间 User-Agent 网络请求 http request allowlist 允许域名 白名单 请求限制 max body method headers 浏览器配置 browser computer use 窗口行为 运行后端 default webdriver sidecar viewport 自动化"
        }
        SystemTab::HttpRequest => {
            "网络请求配置 http request tool allowlist 工具 基础行为 启用 允许域名 白名单 请求限制 max body timeout User-Agent method headers"
        }
        SystemTab::Browser => {
            "浏览器配置 browser computer use tool 工具 基础行为 允许域名 窗口行为 运行后端 default webdriver sidecar viewport 自动化"
        }
    }
}

pub(super) fn system_tab_matches_query(tab: SystemTab, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let needle = query.to_lowercase();
    tab.to_string().to_lowercase().contains(&needle)
        || system_tab_search_text(tab).to_lowercase().contains(&needle)
}

pub(super) fn active_tab_help_modal_open(app: &App, active_tab: SystemTab) -> bool {
    crate::app::components::system_settings_help::help_open_for_tab(
        app.system_settings_help_tab,
        active_tab,
    ) || match active_tab {
        SystemTab::Agents => {
            app.agents_settings.active_config_tab == crate::app::state::AgentConfigTab::Heartbeat
                && app.heartbeat_settings.show_help_modal
        }
        SystemTab::Cron => {
            app.cron_settings.show_help_modal || app.scheduler_settings.show_help_modal
        }
        SystemTab::Reliability => app.reliability_settings.show_help_modal,
        SystemTab::Autonomy => app.autonomy_settings.show_help_modal,
        SystemTab::Security => app.security_settings.show_help_modal,
        SystemTab::GatewayClient => app.gateway_client_settings.show_help_modal,
        SystemTab::Gateway => app.gateway_settings.show_help_modal,
        SystemTab::Observability => app.observability_settings.show_help_modal,
        SystemTab::Proxy => app.proxy_settings.show_help_modal,
        SystemTab::Skills => app.skills_settings.show_help_modal,
        _ => false,
    }
}

pub(super) fn active_tab_save_error(app: &App, active_tab: SystemTab) -> Option<&str> {
    match active_tab {
        SystemTab::Providers => app.provider_settings.save_error.as_deref(),
        SystemTab::Models => match app.model_settings.active_tab {
            crate::app::state::ModelConfigTab::Models => app.model_settings.save_error.as_deref(),
            crate::app::state::ModelConfigTab::ModelRoutes => {
                app.model_routes_settings.save_error.as_deref()
            }
            crate::app::state::ModelConfigTab::QueryClassification => {
                app.query_classification_settings.save_error.as_deref()
            }
            crate::app::state::ModelConfigTab::Multimodal => {
                app.multimodal_settings.save_error.as_deref()
            }
        },
        SystemTab::Cron => app
            .cron_settings
            .save_error
            .as_deref()
            .or(app.scheduler_settings.save_error.as_deref()),
        SystemTab::Agents => match app.agents_settings.active_config_tab {
            crate::app::state::AgentConfigTab::Agents => {
                app.agents_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::Heartbeat => {
                app.heartbeat_settings.save_error.as_deref()
            }
            crate::app::state::AgentConfigTab::DialogueFlow
            | crate::app::state::AgentConfigTab::QuickMessages => None,
        },
        SystemTab::Staff => app.staff_settings.save_error.as_deref(),
        SystemTab::Reliability => app.reliability_settings.save_error.as_deref(),
        SystemTab::Channels => app.channels_settings.save_error.as_deref(),
        SystemTab::Memory => match app.memory_settings.active_tab {
            crate::app::state::MemorySettingsTab::Storage => {
                app.storage_settings.save_error.as_deref()
            }
            _ => app.memory_settings.save_error.as_deref(),
        },
        SystemTab::Runtime => app.runtime_settings.save_error.as_deref(),
        SystemTab::Acp => app.acp_settings.save_error.as_deref(),
        SystemTab::Mcp => app.mcp_settings.save_error.as_deref(),
        SystemTab::Autonomy => app.autonomy_settings.save_error.as_deref(),
        SystemTab::Security => app.security_settings.save_error.as_deref(),
        SystemTab::GatewayClient => app.gateway_client_settings.save_error.as_deref(),
        SystemTab::Gateway => app.gateway_settings.save_error.as_deref(),
        SystemTab::Observability => app.observability_settings.save_error.as_deref(),
        SystemTab::Proxy => app.proxy_settings.save_error.as_deref(),
        SystemTab::Tunnel => app.tunnel_settings.save_error.as_deref(),
        SystemTab::Skills => app.skills_settings.save_error.as_deref(),
        SystemTab::Hooks => app.hooks_settings.save_error.as_deref(),
        SystemTab::WebSearch => app.web_search_settings.save_error.as_deref(),
        SystemTab::HttpRequest => app.http_request_settings.save_error.as_deref(),
        SystemTab::Browser => app.browser_settings.save_error.as_deref(),
        SystemTab::Themes => app.themes_settings.error.as_deref(),
        SystemTab::General | SystemTab::Editor | SystemTab::Projects => None,
    }
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
    let filtered_tabs = SystemTab::all()
        .iter()
        .copied()
        .filter(|tab| system_tab_matches_query(*tab, &app.system_settings_query))
        .collect::<Vec<_>>();
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
                .map(|tab| {
                    // 判断当前标签页是否处于激活状态
                    let is_active = *tab == active_tab
                        || (*tab == SystemTab::WebSearch && is_network_tab(active_tab));
                    // 创建标签页文本标签
                    let label = text(tab.to_string()).size(13);

                    // 构建标签页按钮
                    let btn = button(container(label).width(Length::Fill).padding([8, 12]))
                        .width(Length::Fill)
                        .on_press(Message::Settings(message::SettingsMessage::SystemTabSelected(
                            *tab,
                        )))
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
        SystemTab::Reliability => crate::app::components::system_settings_reliability::view(app),
        SystemTab::Channels => crate::app::components::system_settings_channels::view(app),
        SystemTab::Memory => crate::app::components::system_settings_memory::view(app),
        SystemTab::Runtime => crate::app::components::system_settings_runtime::view(app),
        SystemTab::Acp => crate::app::components::system_settings_acp::view(app),
        SystemTab::Mcp => crate::app::components::system_settings_mcp::view(app),
        SystemTab::Autonomy => crate::app::components::system_settings_autonomy::view(app),
        SystemTab::Security => crate::app::components::system_settings_security::view(app),
        SystemTab::GatewayClient => {
            crate::app::components::system_settings_gateway_client::view(app)
        }
        SystemTab::Gateway => crate::app::components::system_settings_gateway::view(app),
        SystemTab::Observability => {
            crate::app::components::system_settings_observability::view(app)
        }
        SystemTab::Proxy => crate::app::components::system_settings_proxy::view(app),
        SystemTab::Tunnel => crate::app::components::system_settings_tunnel::view(app),
        SystemTab::Skills => crate::app::components::system_settings_skills::view(app),
        SystemTab::Hooks => crate::app::components::system_settings_hooks::view(app),
        SystemTab::WebSearch | SystemTab::HttpRequest | SystemTab::Browser => {
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
                        crate::app::components::system_settings_help::help_button_bar(active_tab)
                    {
                        content_with_help = content_with_help.push(help_bar);
                    }
                    if let Some(err) = active_tab_save_error(app, active_tab) {
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

    if app.system_settings_tab == SystemTab::Agents
        && app.agents_settings.active_config_tab == crate::app::state::AgentConfigTab::Heartbeat
    {
        dialog = crate::app::components::system_settings_heartbeat::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Cron {
        dialog = crate::app::components::system_settings_cron::view_overlays(app, dialog);
    }

    if app.system_settings_tab == SystemTab::Reliability {
        dialog = crate::app::components::system_settings_reliability::view_overlays(app, dialog);
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

    if app.system_settings_tab == SystemTab::Mcp {
        dialog = crate::app::components::system_settings_mcp::view_overlays(app, dialog);
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

    if app.system_settings_tab == SystemTab::Skills {
        dialog = crate::app::components::system_settings_skills::view_overlays(app, dialog);
    }

    dialog = crate::app::components::system_settings_help::with_help_modal(
        app,
        dialog,
        app.system_settings_tab,
        app.system_settings_help_tab,
    );

    dialog
}
