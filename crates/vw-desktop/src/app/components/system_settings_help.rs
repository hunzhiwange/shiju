//! 系统设置中 help 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings::SystemTab;
use crate::app::components::system_settings_common::{
    settings_help_button, with_settings_help_modal,
};
use crate::app::message::settings::SettingsMessage;
use crate::app::{App, Message};
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

const GENERAL_HELP: &str = r#"常规设置说明

一、作用
- 本页控制桌面应用自身的显示、终端与预览体验，不会改动后端代理运行策略。
- 适合先确定主题、自动保存模式以及终端字体等长期偏好。

二、主要项
1) 应用主题
- 控制桌面界面的整体配色。
2) 自动保存
- 控制预览编辑区何时自动保存。
3) 终端 Shell / 主题 / 字体 / 字号
- 影响内置终端的启动环境与可读性。

三、建议
- Shell 尽量与日常开发环境保持一致，减少脚本差异。
- 字体优先选择等宽字体，便于长时间阅读终端输出。
- 如果预览区经常误保存，优先使用更保守的自动保存模式。
"#;

const EDITOR_HELP: &str = r#"编辑器设置说明

一、作用
- 本页控制桌面编辑器的主题、字号与行高表现。
- 它影响代码阅读与编辑体验，但不会修改文件内容。

二、主要项
1) 跟随系统主题
- 启用后自动与应用主题保持一致。
2) 编辑器主题
- 仅在关闭“跟随系统主题”后生效。
3) 字体大小 / 行高 / 自动行高
- 共同决定代码显示密度与可读性。

三、建议
- 长时间编码建议先调好字号，再决定是否开启自动行高。
- 如果代码行较长或屏幕较小，适当增大行高更利于扫读。
"#;

const PROJECTS_HELP: &str = r#"项目设置说明

一、作用
- 本页管理桌面端最近项目入口、当前项目 Worktree 开关，以及快捷打开项目。
- 这些设置只影响本机工作流，不会写入服务器端模型配置。

二、主要项
1) 打开文件夹
- 选择新的工作区目录并切换当前项目。
2) 当前项目 Worktree
- 控制该项目创建新会话时是否允许选择或创建 Git worktree。
3) 历史项目
- 支持重命名显示名称、删除入口和再次打开。

三、建议
- 只有经常需要并行分支开发的项目才建议开启 Worktree。
- 删除历史项目入口不会删除磁盘上的真实目录。
"#;

const PROVIDERS_HELP: &str = r#"模型提供商说明

一、作用
- 本页管理模型提供商的连接状态、热门提供商接入与自定义提供商入口。
- Provider 决定模型列表、鉴权方式以及后续模型选择来源。

二、主要操作
1) 已连接提供商
- 查看当前已接入的提供商，支持修改密钥、编辑和断开连接。
2) 热门提供商
- 从常用目录中快速连接 OpenAI、Anthropic、Gemini 等提供商。
3) 自定义提供商
- 用于接入 OpenAI 兼容接口或内部网关。

三、建议
- 先连 Provider，再到“模型”页按需启用模型。
- 自定义提供商前先确认 base URL、鉴权头和模型 ID 命名约定。
"#;

const MODELS_HELP: &str = r#"模型设置说明

一、作用
- 本页展示已连接提供商下的模型清单，并控制模型是否在应用内可用。
- 它适合做模型筛选、禁用无效模型与查看能力元信息。

二、主要操作
1) 刷新
- 从当前已连接提供商重新拉取模型列表。
2) 搜索
- 按提供商名、模型名或模型 ID 过滤。
3) 启用开关
- 控制模型是否出现在后续模型选择与路由候选中。
4) 更多
- 查看上下文长度、工具调用与附件支持等详细信息。
5) 多模态
- 配置图片数量与大小上限、远程抓取、图片直传和 OCR 图片模型。

三、建议
- 只保留实际会用到的模型，减少选择噪音与误用风险。
- Provider 更新后先刷新一次，再检查模型启用状态是否符合预期。
"#;

const AGENTS_HELP: &str = r#"智能体配置说明

一、作用
- 本页管理主代理与委托代理的模型、提示词、工具权限和执行边界。
- 这里决定不同角色代理如何协作，以及各自能做什么。

二、主要区域
1) 代理列表
- 选择已有代理、创建新代理并查看当前启停状态。
2) 基础配置
- 选择 provider、model、兼容模式、温度与最大迭代次数。
3) Prompt / Identity
- 编辑系统提示词与工作区身份文件。
4) 工具权限
- 控制允许的工具集合、并行能力与深度限制。
5) 对话流
- 控制时间线中的推理摘要、shell 与编辑工具默认展开方式。
6) 快捷消息 / 心跳配置
- 管理聊天输入区快捷操作，以及周期心跳的间隔、兜底消息和投递目标。

三、建议
- 主代理保持收敛，复杂职责下沉给专门子代理。
- 自定义代理前先明确角色边界，避免多个代理能力重叠。
"#;

const CHANNELS_HELP: &str = r#"通道配置说明

一、作用
- 本页集中管理 CLI 与外部消息通道的启用状态、鉴权信息和接收模式。
- 适合统一维护 Telegram、Slack、Webhook 等多入口集成。

二、主要区域
1) 全局参数
- 包括固定项目目录和消息超时预算。
2) 已启用通道
- 快速查看当前开放的入口。
3) 各通道面板
- 每个通道独立维护 token、URL、白名单、群聊回复模式等字段。

三、建议
- 只展开并启用正在使用的通道，减少误配置。
- 敏感 token 修改后，优先做一次真实消息链路验证。
"#;

const MEMORY_HELP: &str = r#"记忆系统说明

一、作用
- 本页控制记忆后端、保留策略、缓存、快照以及 embedding 检索参数。
- 它直接影响长期记忆质量、磁盘占用和检索效果。

二、主要区域
1) 基础行为
- 控制自动保存、卫生清理、响应缓存、快照与自动恢复。
2) 保留与缓存
- 管理归档/清理天数、SQLite 超时、缓存容量。
3) 嵌入与检索
- 配置 embedding provider、模型、维度和混合检索权重。
- 嵌入模型支持 hint:模式名 格式引用"嵌入路由"中定义的路由规则。
4) 嵌入路由
- 按匹配模式配置不同场景的 embedding provider、模型、维度与 API Key。
- 在“嵌入与检索”中填写 hint:模式名 即可引用相应路由。
5) 记忆存储
- 配置 PostgreSQL、MariaDB 或 SQLite 记忆后端及远程数据库连接参数。
- Schema、数据表、连接超时和 TLS 主要用于远程 SQL 存储。
6) Qdrant
- 在使用 Qdrant 时配置 URL、collection 与 API key。

三、建议
- embedding 维度与向量库索引必须保持一致。
- 先确定保留天数，再逐步调缓存容量，避免无意义的存储膨胀。
- 需要按场景使用不同嵌入模型时，先在"嵌入路由"中定义路由，再在此处用 hint:模式名 引用。
"#;

const RUNTIME_HELP: &str = r#"运行时配置说明

一、作用
- 本页控制代理执行环境的类型，以及 Docker / WASM 的资源与安全边界。
- 运行时配置直接决定工具隔离能力、性能成本和可访问资源范围。

二、主要区域
1) 运行时类型
- 在 native、docker、wasm 之间选择执行环境。
2) Docker 配置
- 管理镜像、网络、内存、CPU、只读根文件系统和工作区挂载。
3) WASM 配置
- 管理工具目录、燃料、内存、模块大小和宿主安全策略。
4) 推理覆盖
- 覆盖 reasoning_enabled / reasoning_level 等运行时提示。

三、建议
- 对高风险工具优先使用 docker 或 wasm，并显式收紧读写范围。
- 修改 runtime kind 后，最好回归一次关键工具链路。
"#;

const TUNNEL_HELP: &str = r#"隧道配置说明

一、作用
- 本页为网关配置公网暴露方式，支持 Cloudflare、Tailscale、ngrok 或自定义命令。
- 它只负责把本地网关映射出去，不替代网关本身的鉴权与限流。

二、主要模式
1) cloudflare
- 使用 Zero Trust token 建立受管隧道。
2) tailscale
- 在 tailnet 内使用 serve，或通过 funnel 暴露到公网。
3) ngrok
- 使用 auth token，可选绑定自定义域名。
4) custom
- 通过自定义启动命令、健康检查和 URL 模式接入其他隧道工具。

三、建议
- 公网暴露前先确认网关自身已开启必要的认证和配对策略。
- 自定义命令建议先在终端单独验证，再写入这里。
"#;

const HOOKS_HELP: &str = r#"钩子配置说明

一、作用
- 本页控制运行时 hooks 总开关、内置 hooks 与可信插件 hooks 的启停状态。
- Hooks 更偏向审计、观测和流程插桩，而不是直接业务逻辑。

二、主要项
1) 总开关
- 关闭后暂停 hooks 执行，但保留单项开关状态。
2) command_logger
- 记录命令调用信息，方便审计与回溯。
3) 插件 hook 审核
- 页面展示已发现插件的事件、命令、参数、工作目录与当前信任状态。
- 信任或撤销操作只提交插件 ID 与服务端重新解析后的稳定哈希，不提交命令内容。
4) 审计记录
- 只记录时间、工具名、耗时和成功状态。
- 不记录工具参数、结果正文、令牌或密钥。

三、建议
- 先启用总开关，再逐项开启真正需要的内置 hook。
- 只信任来源明确且命令内容已经人工审核的插件 hook；声明变化后应重新审核。
"#;

const WEB_SEARCH_HELP: &str = r#"网页搜索配置说明

一、作用
- web_search 用于让代理主动检索公开网页搜索结果。
- 当前支持 duckduckgo、brave、serper、google、bing。

二、主要字段
1) enabled / provider
- 控制工具开关并选择搜索提供方。
2) api_key / brave_api_key / api_url
- 配置提供方鉴权与可选的兼容端点。
3) max_results / timeout_secs / user_agent
- 控制结果数量、请求超时和默认请求标识。

三、建议
- DuckDuckGo 无需 API Key；其他提供方应按需配置密钥。
- 自定义 api_url 前先确认端点协议与鉴权方式兼容。
"#;

const HTTP_REQUEST_HELP: &str = r#"网络请求配置说明

一、作用
- 本页控制 http_request 工具是否可用，以及外部请求的白名单与限制。
- 它是网络访问边界的重要一层，不建议长期放开所有域名。

二、主要字段
1) 启用
- 控制 http_request 工具是否能被调用。
2) 响应大小上限 / 超时时间 / User-Agent
- 限制请求资源消耗并设置默认请求标识。
3) 允许域名
- 维护白名单；为空时默认拒绝全部外部请求。

三、建议
- 只放行业务必需域名，尽量避免使用 `*`。
- 较大的响应体会抬高内存占用与模型上下文成本。
"#;

const BROWSER_HELP: &str = r#"浏览器配置说明

一、作用
- 本页控制 browser / browser_open 工具的启用状态、域名边界与自动化后端。
- 它同时覆盖轻量页面打开和系统级浏览器控制能力。

二、主要区域
1) 基础行为
- 包括启用开关、允许域名、打开方式、会话名称与后端类型。
2) Native 后端
- 使用 WebDriver 驱动本地 Chrome/Chromium。
3) Computer Use 后端
- 使用 sidecar 执行鼠标、键盘、截图等系统级动作。

三、建议
- 先限制 allowed_domains，再决定是否启用更高权限的后端。
- `computer_use.allow_remote_endpoint` 仅在可信内网环境下开启。
"#;

fn help_content(tab: SystemTab) -> Option<(&'static str, &'static str)> {
    match tab {
        SystemTab::General => Some(("常规设置帮助", GENERAL_HELP)),
        SystemTab::Editor => Some(("编辑器设置帮助", EDITOR_HELP)),
        SystemTab::Projects => Some(("项目设置帮助", PROJECTS_HELP)),
        SystemTab::Providers => Some(("模型提供商帮助", PROVIDERS_HELP)),
        SystemTab::Models => Some(("模型设置帮助", MODELS_HELP)),
        SystemTab::Agents => Some(("智能体配置帮助", AGENTS_HELP)),
        SystemTab::Channels => Some(("通道配置帮助", CHANNELS_HELP)),
        SystemTab::Memory => Some(("记忆系统帮助", MEMORY_HELP)),
        SystemTab::Runtime => Some(("运行时配置帮助", RUNTIME_HELP)),
        SystemTab::Tunnel => Some(("隧道配置帮助", TUNNEL_HELP)),
        SystemTab::Hooks => Some(("钩子配置帮助", HOOKS_HELP)),
        SystemTab::WebSearch => Some(("网页搜索帮助", WEB_SEARCH_HELP)),
        SystemTab::HttpRequest => Some(("网络请求帮助", HTTP_REQUEST_HELP)),
        SystemTab::Browser => Some(("浏览器配置帮助", BROWSER_HELP)),
        _ => None,
    }
}

/// 构建或处理 `help_open_for_tab` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn help_open_for_tab(open_tab: Option<SystemTab>, active_tab: SystemTab) -> bool {
    open_tab == Some(active_tab) && help_content(active_tab).is_some()
}

/// 构建或处理 `help_button_bar` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn help_button_bar(tab: SystemTab) -> Option<Element<'static, Message>> {
    help_content(tab)?;

    Some(
        row![
            container(text(" ")).width(Length::Fill),
            settings_help_button(Message::Settings(SettingsMessage::SystemHelpOpen(tab))),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// 构建或处理 `with_help_modal` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn with_help_modal<'a>(
    app: &App,
    base: Element<'a, Message>,
    active_tab: SystemTab,
    open_tab: Option<SystemTab>,
) -> Element<'a, Message> {
    if open_tab != Some(active_tab) {
        return base;
    }

    let Some((title, help_text)) = help_content(active_tab) else {
        return base;
    };

    with_settings_help_modal(
        app,
        base,
        title,
        help_text,
        Message::Settings(SettingsMessage::SystemHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_help_tests.rs"]
mod system_settings_help_tests;
