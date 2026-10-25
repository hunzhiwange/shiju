//! 显式命令目录：解析参数和聊天帮助共用，未接通能力不注册。
pub(crate) struct CommandMetadata {
    pub name: &'static str,
    pub group: &'static str,
    pub min_args: usize,
    pub max_args: usize,
    pub usage: &'static str,
    pub description: &'static str,
    pub example: &'static str,
    pub aliases: &'static [&'static str],
    pub approval: bool,
    pub errors: &'static str,
}

pub(crate) const COMMANDS: &[CommandMetadata] = &[
    CommandMetadata {
        name: "projects",
        group: "项目",
        min_args: 0,
        max_args: 0,
        usage: "/projects",
        description: "列出当前租户可访问项目的稳定 ID",
        example: "/projects",
        aliases: &[],
        approval: false,
        errors: "失效目录不显示",
    },
    CommandMetadata {
        name: "project",
        group: "项目",
        min_args: 0,
        max_args: 1,
        usage: "/project [稳定ID|default]",
        description: "查看或切换当前聊天项目；default 恢复实例默认",
        example: "/project default",
        aliases: &[],
        approval: false,
        errors: "未知 ID 或聊天繁忙时不切换",
    },
    CommandMetadata {
        name: "new",
        group: "会话",
        min_args: 0,
        max_args: 0,
        usage: "/new",
        description: "立即创建新会话并返回 ID，保留旧会话及当前项目",
        example: "/new",
        aliases: &["session"],
        approval: false,
        errors: "项目解析失败不清历史",
    },
    CommandMetadata {
        name: "clear",
        group: "会话",
        min_args: 0,
        max_args: 0,
        usage: "/clear",
        description: "重置当前上下文，下一条消息创建新会话；旧历史保留",
        example: "/clear",
        aliases: &[],
        approval: false,
        errors: "繁忙或项目不可用时拒绝；失败保留历史和绑定",
    },
    CommandMetadata {
        name: "sessions",
        group: "会话",
        min_args: 0,
        max_args: 0,
        usage: "/sessions",
        description: "列出当前聊天和项目拥有的非归档会话；/resume <ID> 恢复",
        example: "/sessions",
        aliases: &[],
        approval: false,
        errors: "繁忙、无所有权或状态变化时拒绝；失败保留当前绑定",
    },
    CommandMetadata {
        name: "resume",
        group: "会话",
        min_args: 0,
        max_args: 1,
        usage: "/resume [ID]",
        description: "无参数列出候选，按完整 ID 恢复当前域拥有的会话",
        example: "/resume ses_example",
        aliases: &[],
        approval: false,
        errors: "繁忙、无所有权或状态变化时拒绝；失败保留当前绑定",
    },
    CommandMetadata {
        name: "continue",
        group: "会话",
        min_args: 0,
        max_args: 0,
        usage: "/continue",
        description: "恢复当前聊天和项目最近的非归档会话",
        example: "/continue",
        aliases: &[],
        approval: false,
        errors: "繁忙、无所有权或状态变化时拒绝；失败保留当前绑定",
    },
    CommandMetadata {
        name: "rename",
        group: "会话",
        min_args: 1,
        max_args: usize::MAX,
        usage: "/rename <标题>",
        description: "重命名当前会话；标题最多 100 字符",
        example: "/rename 调试记录",
        aliases: &[],
        approval: false,
        errors: "繁忙、无所有权或状态变化时拒绝；失败保留当前绑定",
    },
    CommandMetadata {
        name: "archive",
        group: "会话",
        min_args: 0,
        max_args: 2,
        usage: "/archive [confirm <当前会话ID>]",
        description: "先请求确认，再按当前会话 ID 归档，保留历史",
        example: "/archive",
        aliases: &[],
        approval: false,
        errors: "繁忙、无所有权或状态变化时拒绝；失败保留当前绑定",
    },
    CommandMetadata {
        name: "fork",
        group: "会话",
        min_args: 0,
        max_args: 1,
        usage: "/fork [message-id]",
        description: "按真实消息锚点分叉并切换会话；无参数列出可用锚点",
        example: "/fork",
        aliases: &[],
        approval: false,
        errors: "外部锚点、跨域、繁忙及过期确认被拒绝，失败保留当前绑定",
    },
    CommandMetadata {
        name: "compact",
        group: "会话",
        min_args: 0,
        max_args: 2,
        usage: "/compact [message-id | confirm message-id]",
        description: "压缩锚点之前的历史，需显式 confirm 确认；无参数列出可用锚点",
        example: "/compact",
        aliases: &["summarize"],
        approval: false,
        errors: "外部锚点、跨域、繁忙及过期确认被拒绝，失败保留当前绑定",
    },
    CommandMetadata {
        name: "reset",
        group: "会话",
        min_args: 0,
        max_args: 2,
        usage: "/reset [message-id | confirm message-id]",
        description: "回退聊天历史及生成的文件变更，需 confirm；无参数列出可用锚点",
        example: "/reset",
        aliases: &["rewind"],
        approval: false,
        errors: "外部锚点、跨域、繁忙及过期确认被拒绝，失败保留当前绑定",
    },
    CommandMetadata {
        name: "settings",
        group: "模型设置",
        min_args: 0,
        max_args: 0,
        usage: "/settings",
        description: "只读查看当前会话设置，不提升权限",
        example: "/settings",
        aliases: &[],
        approval: false,
        errors: "参数不合法时保留全部设置",
    },
    CommandMetadata {
        name: "models",
        group: "模型设置",
        min_args: 0,
        max_args: 1,
        usage: "/models [provider]",
        description: "查看当前 provider 真实模型目录；可保留旧写法切换 provider",
        example: "/models",
        aliases: &["providers"],
        approval: false,
        errors: "provider 不存在或初始化失败不切换",
    },
    CommandMetadata {
        name: "model",
        group: "模型设置",
        min_args: 0,
        max_args: 1,
        usage: "/model [ID]",
        description: "查看当前模型或选择当前 provider 目录中的模型",
        example: "/model gpt-5",
        aliases: &[],
        approval: false,
        errors: "目录未声明的模型被拒绝，历史和路由不变",
    },
    CommandMetadata {
        name: "effort",
        group: "模型设置",
        min_args: 0,
        max_args: 1,
        usage: "/effort [level]",
        description: "查看可用推理强度或切换；default 恢复默认",
        example: "/effort high",
        aliases: &[],
        approval: false,
        errors: "只接受当前模型真实支持的强度，失败保持原状态",
    },
    CommandMetadata {
        name: "help",
        group: "执行控制",
        min_args: 0,
        max_args: 1,
        usage: "/help [命令名]",
        description: "查看全部已接通命令或详情，不调用模型",
        example: "/help model",
        aliases: &[],
        approval: false,
        errors: "未知名称会提示返回 /help",
    },
    CommandMetadata {
        name: "status",
        group: "执行控制",
        min_args: 0,
        max_args: 0,
        usage: "/status",
        description: "查看当前项目、会话、模型、强度与运行状态",
        example: "/status",
        aliases: &[],
        approval: false,
        errors: "项目不可用时报告错误",
    },
    CommandMetadata {
        name: "task",
        group: "执行控制",
        min_args: 0,
        max_args: 0,
        usage: "/task",
        description: "重置当前上下文并进入任务模式，后续文本创建任务",
        example: "/task",
        aliases: &[],
        approval: false,
        errors: "项目解析失败不进入任务模式",
    },
    CommandMetadata {
        name: "approve-all-once",
        group: "授权",
        min_args: 0,
        max_args: 0,
        usage: "/approve-all-once",
        description: "创建一次性全工具授权请求；随后用 /approve-confirm apr-xxxxxx 确认",
        example: "/approve-all-once",
        aliases: &[],
        approval: true,
        errors: "仍遵守授权策略，不能绕过审批人检查",
    },
    CommandMetadata {
        name: "approve-request",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/approve-request <tool-name>",
        description: "请求单工具授权；随后用 /approve-confirm apr-xxxxxx 确认",
        example: "/approve-request shell",
        aliases: &[],
        approval: true,
        errors: "未知工具或策略拒绝时不会授权",
    },
    CommandMetadata {
        name: "approve-confirm",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/approve-confirm <request-id>",
        description: "确认待授权请求，按现有策略保存结果",
        example: "/approve-confirm apr-xxxxxx",
        aliases: &[],
        approval: true,
        errors: "过期、跨聊天或不存在的请求会被拒绝",
    },
    CommandMetadata {
        name: "approve-allow",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/approve-allow <request-id>",
        description: "批准待审批请求，仅限本次调用",
        example: "/approve-allow apr-xxxxxx",
        aliases: &[],
        approval: true,
        errors: "请求必须属于当前授权上下文",
    },
    CommandMetadata {
        name: "approve-deny",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/approve-deny <request-id>",
        description: "拒绝当前待审批请求",
        example: "/approve-deny apr-xxxxxx",
        aliases: &[],
        approval: true,
        errors: "不接受其他聊天的请求",
    },
    CommandMetadata {
        name: "approve-pending",
        group: "授权",
        min_args: 0,
        max_args: 0,
        usage: "/approve-pending",
        description: "查看当前作用域内待处理授权",
        example: "/approve-pending",
        aliases: &[],
        approval: true,
        errors: "只列出当前发送者和聊天的请求",
    },
    CommandMetadata {
        name: "approve",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/approve <tool-name>",
        description: "直接批准单个受监督工具，仍受授权策略限制",
        example: "/approve shell",
        aliases: &[],
        approval: true,
        errors: "未知工具和禁止授权的工具均拒绝",
    },
    CommandMetadata {
        name: "unapprove",
        group: "授权",
        min_args: 1,
        max_args: 1,
        usage: "/unapprove <tool-name>",
        description: "撤销单个工具批准",
        example: "/unapprove shell",
        aliases: &[],
        approval: true,
        errors: "参数错误不改变权限",
    },
    CommandMetadata {
        name: "approvals",
        group: "授权",
        min_args: 0,
        max_args: 0,
        usage: "/approvals",
        description: "查看现有授权配置与当前运行状态",
        example: "/approvals",
        aliases: &[],
        approval: true,
        errors: "发送者无权限时拒绝查询",
    },
];

pub(crate) fn find_command(name: &str) -> Option<&'static CommandMetadata> {
    COMMANDS.iter().find(|meta| meta.name == name || meta.aliases.contains(&name))
}

fn entry(meta: &CommandMetadata, approval_allowed: bool) -> String {
    let permission = if meta.approval {
        if approval_allowed {
            "需要授权管理权限（仍按现有策略检查）"
        } else {
            "需要授权管理权限；当前发送者无权执行"
        }
    } else {
        "当前通道允许的发送者"
    };
    let aliases = if meta.aliases.is_empty() {
        "无".to_string()
    } else {
        meta.aliases.iter().map(|name| format!("/{name}")).collect::<Vec<_>>().join("、")
    };
    format!(
        "{}\n用途：{}\n示例：{}\n解析别名：{}\n权限：{}\n错误：{}；参数错误时返回用法。",
        meta.usage, meta.description, meta.example, aliases, permission, meta.errors
    )
}

/// 按完整命令条目打包，保守字节上限低于文本平台限制，交由原通道 send 发送。
pub(crate) fn help_messages(query: &str, approval_allowed: bool) -> Vec<String> {
    let query = query.trim_start_matches('/').to_ascii_lowercase();
    if !query.is_empty() {
        return vec![match find_command(&query) {
            Some(meta) => entry(meta, approval_allowed),
            None => format!("未知或未接通命令：{query}。使用 /help 查看全部支持命令。"),
        }];
    }
    let mut messages = Vec::new();
    let mut chunk = String::from("当前实例全部已接通命令（/help <命令名> 查看详情）：");
    let mut group = "";
    for meta in COMMANDS {
        let heading =
            if group != meta.group { format!("\n【{}】\n", meta.group) } else { String::new() };
        let item = format!("{heading}{}", entry(meta, approval_allowed));
        if chunk.len() + item.len() + 2 > 1800 {
            messages.push(std::mem::take(&mut chunk));
        }
        if !chunk.is_empty() {
            chunk.push_str("\n\n");
        }
        chunk.push_str(&item);
        group = meta.group;
    }
    if !chunk.is_empty() {
        messages.push(chunk);
    }
    messages
}
