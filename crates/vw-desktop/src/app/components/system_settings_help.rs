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

const DIALOGUE_FLOW_HELP: &str = r#"对话流设置说明

一、作用
- 本页控制聊天时间线的展示细节与跟进消息处理方式。
- 它只影响桌面端交互体验，不改动模型能力或系统安全策略。

二、主要项
1) 显示推理摘要
- 决定是否在时间线中展示模型推理摘要。
2) 展开 shell / 编辑工具
- 控制相关工具块在消息中默认展开还是折叠。

三、建议
- 对长会话或代码任务，开启推理摘要更利于快速回看。
- 如果工具调用很多，默认折叠可降低时间线噪音。
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

三、建议
- 只保留实际会用到的模型，减少选择噪音与误用风险。
- Provider 更新后先刷新一次，再检查模型启用状态是否符合预期。
"#;

const EMBEDDING_ROUTES_HELP: &str = r#"嵌入路由说明

一、作用
- 本页为 embedding 请求配置"匹配模式 -> 提供商 / 模型 / 维度"的路由规则。
- 当系统需要向量化文本时，会按这里的规则选择具体 embedding 模型。

二、主要字段
1) 匹配模式
- 用于标识某类 embedding 用途，例如 semantic、memory、rag。
2) 提供商 / 模型
- 指向实际执行 embedding 的 provider 与 model。
3) 维度
- 可选覆盖向量维度；留空时使用模型默认值。

三、用法
- 在"记忆配置"页面的"嵌入模型"字段中，使用 hint:模式名 格式引用路由。
- 例如：定义了匹配模式为 semantic 的路由后，在记忆配置中填写 hint:semantic 即可。
- 系统会自动查找匹配的路由，用路由中的 provider/model/dimensions 替换默认值。
- 如果不使用 hint: 前缀，则直接使用记忆配置中的默认嵌入设置。

四、建议
- 同一模式尽量固定到单一模型，避免向量空间不兼容。
- 修改维度前先确认目标后端与现有索引是否支持重建。
"#;

const MODEL_ROUTES_HELP: &str = r#"模型路由说明

一、作用
- 本页根据 pattern 为请求选择 provider / model，并设置优先级。
- 它适合把 code、reasoning、fast 等不同任务定向到不同模型。

二、主要字段
1) 匹配模式
- 描述某类请求意图或标签。
2) 提供商 / 模型
- 指定匹配后应使用的实际模型。
3) 优先级
- 数值越大越优先；保存时会同步到查询分类链路。

三、建议
- 先保证 pattern 命名稳定，再逐步细化优先级。
- 避免多个高优先级规则表达同一意图，否则排查命中链路会变复杂。
"#;

const QUERY_CLASSIFICATION_HELP: &str = r#"查询模型分类说明

一、作用
- 本页把用户请求 pattern 映射到已配置的模型路由，供后续模型选择使用。
- 模型分类链路更偏“请求理解”，不是直接执行模型调用。

二、主要字段
1) 启用开关
- 控制查询模型分类是否参与决策。
2) pattern
- 用于匹配用户请求或路由线索。
3) 模型分类
- 从模型路由中选择；匹配成功后使用同名路由的 provider / model。
4) priority
- 用于解决多条规则同时命中的优先顺序。

三、建议
- 先在“模型路由”创建路由，再在本页选择对应模型分类。
- 规则过多时优先收敛重复 pattern，减少维护成本。
"#;

const AGENTS_HELP: &str = r#"委托代理配置说明

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
4) Qdrant
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

const STORAGE_HELP: &str = r#"存储配置说明

一、作用
- 本页配置持久化存储 provider、连接地址、schema、table 与 TLS。
- 它主要服务于数据库型记忆或远程持久化场景。

二、主要字段
1) 存储类型
- 指定后端类型，例如 postgres、mariadb、sqlite。
2) 数据库地址
- 远程 SQL 存储的连接串；留空时不写入地址。
3) Schema / 数据表
- 控制实际读写位置。
4) 连接超时 / TLS
- 控制连接等待时长和传输加密。

三、建议
- 生产环境优先开启 TLS，并使用最小权限数据库账号。
- 变更 schema 或 table 前先确认现有数据迁移策略。
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
- 本页控制运行时 hooks 总开关与内置 hooks 的启停状态。
- Hooks 更偏向审计、观测和流程插桩，而不是直接业务逻辑。

二、主要项
1) 总开关
- 关闭后暂停 hooks 执行，但保留单项开关状态。
2) command_logger
- 记录命令调用信息，方便审计与回溯。
3) 自定义 hooks 预留
- 当前仅做未来扩展占位。

三、建议
- 先启用总开关，再逐项开启真正需要的内置 hook。
- 涉及敏感命令审计时，注意不要把密钥等内容写入日志。
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

const MULTIMODAL_HELP: &str = r#"多模态配置说明

一、作用
- 本页控制单次请求允许携带的图片数量、大小上限，以及是否允许抓取远程图片。
- 它直接影响上传成本、带宽与模型输入安全边界。

二、主要项
1) 最大图片数量
- 限制单次请求最多附带多少张图。
2) 单张图片大小上限
- 超过上限的图片应在进入模型前压缩或拒绝。
3) 允许远程抓取
- 控制是否允许从 http/https URL 拉取远程图片。
4) 模型支持图片
- 默认开启，优先让当前模型直接处理图片；即使启用 OCR，也不会先转换为 OCR 文本。
5) 图片文字提取（OCR）
- 关闭“模型支持图片”后，才会使用所选图片模型提取文字。

三、建议
- 先从较小上限开始，按真实使用量逐步放宽。
- 远程抓取开启后，最好同时配合网络域名白名单策略。
"#;

fn help_content(tab: SystemTab) -> Option<(&'static str, &'static str)> {
    match tab {
        SystemTab::General => Some(("常规设置帮助", GENERAL_HELP)),
        SystemTab::DialogueFlow => Some(("对话流设置帮助", DIALOGUE_FLOW_HELP)),
        SystemTab::Editor => Some(("编辑器设置帮助", EDITOR_HELP)),
        SystemTab::Projects => Some(("项目设置帮助", PROJECTS_HELP)),
        SystemTab::Providers => Some(("模型提供商帮助", PROVIDERS_HELP)),
        SystemTab::Models => Some(("模型设置帮助", MODELS_HELP)),
        SystemTab::EmbeddingRoutes => Some(("嵌入路由帮助", EMBEDDING_ROUTES_HELP)),
        SystemTab::ModelRoutes => Some(("模型路由帮助", MODEL_ROUTES_HELP)),
        SystemTab::QueryClassification => Some(("查询模型分类帮助", QUERY_CLASSIFICATION_HELP)),
        SystemTab::Agents => Some(("委托代理配置帮助", AGENTS_HELP)),
        SystemTab::Channels => Some(("通道配置帮助", CHANNELS_HELP)),
        SystemTab::Memory => Some(("记忆系统帮助", MEMORY_HELP)),
        SystemTab::Runtime => Some(("运行时配置帮助", RUNTIME_HELP)),
        SystemTab::Storage => Some(("存储配置帮助", STORAGE_HELP)),
        SystemTab::Tunnel => Some(("隧道配置帮助", TUNNEL_HELP)),
        SystemTab::Hooks => Some(("钩子配置帮助", HOOKS_HELP)),
        SystemTab::HttpRequest => Some(("网络请求帮助", HTTP_REQUEST_HELP)),
        SystemTab::Browser => Some(("浏览器配置帮助", BROWSER_HELP)),
        SystemTab::Multimodal => Some(("多模态配置帮助", MULTIMODAL_HELP)),
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
