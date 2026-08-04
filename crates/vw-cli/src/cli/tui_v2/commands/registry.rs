use std::collections::{HashMap, HashSet};

use crate::cli::tui_v2::input::TuiSlashCommandKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CommandSource {
    Mcp,
    Plugin,
    Skill,
    Workflow,
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAction {
    Builtin(TuiSlashCommandKind),
    Gateway(String),
    PromptExpansion(String),
    LocalEffect(String),
    ExternalEffect(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAvailability {
    Available,
    Unavailable { reason: String },
}

impl CommandAvailability {
    pub(crate) fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub(crate) fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unavailable { reason } => Some(reason.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandExecutionPolicy {
    Immediate,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSpec {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) summary: String,
    pub(crate) takes_argument: bool,
    pub(crate) argument_hint: Option<String>,
    pub(crate) source: CommandSource,
    pub(crate) availability: CommandAvailability,
    pub(crate) execution_policy: CommandExecutionPolicy,
    pub(crate) action: CommandAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandInvocation {
    pub(crate) name: String,
    pub(crate) argument: Option<String>,
    pub(crate) source: CommandSource,
    pub(crate) execution_policy: CommandExecutionPolicy,
    pub(crate) action: CommandAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandRegistry {
    commands: Vec<CommandSpec>,
    source_errors: Vec<String>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl CommandRegistry {
    pub(crate) fn builtin() -> Self {
        Self::merge(vec![(CommandSource::Builtin, Ok(builtin_commands()))])
    }

    pub(crate) fn merge(sources: Vec<(CommandSource, Result<Vec<CommandSpec>, String>)>) -> Self {
        let mut by_name: HashMap<String, CommandSpec> = HashMap::new();
        let mut errors = Vec::new();
        for (source, result) in sources {
            let specs = match result {
                Ok(specs) => specs,
                Err(error) => {
                    errors.push(format!("{source:?}: {}", sanitize_registry_error(&error)));
                    continue;
                }
            };
            for mut spec in specs {
                spec.source = source;
                if source != CommandSource::Builtin
                    && matches!(
                        spec.action,
                        CommandAction::Builtin(_)
                            | CommandAction::LocalEffect(_)
                            | CommandAction::ExternalEffect(_)
                    )
                {
                    errors.push(format!(
                        "{source:?}: /{} cannot register privileged local effect",
                        spec.name
                    ));
                    continue;
                }
                let key = spec.name.to_ascii_lowercase();
                let replace = by_name.get(&key).is_none_or(|current| source > current.source);
                if replace {
                    by_name.insert(key, spec);
                }
            }
        }
        let mut commands = by_name.into_values().collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            right.source.cmp(&left.source).then_with(|| left.name.cmp(&right.name))
        });
        deduplicate_aliases(&mut commands);
        Self { commands, source_errors: errors }
    }

    pub(crate) fn commands(&self) -> &[CommandSpec] {
        &self.commands
    }

    pub(crate) fn errors(&self) -> &[String] {
        &self.source_errors
    }

    pub(crate) fn matching_commands(&self, partial: &str) -> Vec<&CommandSpec> {
        let partial = partial.trim().trim_start_matches('/').to_ascii_lowercase();
        let mut commands = self
            .commands
            .iter()
            .filter(|spec| {
                partial.is_empty()
                    || spec.name.to_ascii_lowercase().starts_with(partial.as_str())
                    || spec
                        .aliases
                        .iter()
                        .any(|alias| alias.to_ascii_lowercase().starts_with(partial.as_str()))
            })
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            command_score(right)
                .cmp(&command_score(left))
                .then_with(|| right.source.cmp(&left.source))
                .then_with(|| left.name.cmp(&right.name))
        });
        commands
    }

    pub(crate) fn resolve(&self, token: &str) -> Option<&CommandSpec> {
        self.commands.iter().find(|spec| {
            spec.name.eq_ignore_ascii_case(token)
                || spec.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(token))
        })
    }

    pub(crate) fn dispatch(
        &self,
        token: &str,
        argument: Option<String>,
    ) -> Result<CommandInvocation, String> {
        let spec = self.resolve(token).ok_or_else(|| format!("unknown slash command: /{token}"))?;
        if let Some(reason) = spec.availability.unavailable_reason() {
            return Err(format!("/{} is unavailable: {reason}", spec.name));
        }
        if argument.is_some() && !spec.takes_argument {
            return Err(format!("/{} does not accept arguments", spec.name));
        }
        Ok(CommandInvocation {
            name: spec.name.clone(),
            argument,
            source: spec.source,
            execution_policy: spec.execution_policy,
            action: spec.action.clone(),
        })
    }

    pub(crate) fn replace_source(
        &self,
        source: CommandSource,
        replacement: Result<Vec<CommandSpec>, String>,
    ) -> Self {
        let mut sources = [
            CommandSource::Mcp,
            CommandSource::Plugin,
            CommandSource::Skill,
            CommandSource::Workflow,
            CommandSource::Builtin,
        ]
        .into_iter()
        .filter(|candidate| *candidate != source)
        .map(|candidate| {
            (
                candidate,
                Ok(self
                    .commands
                    .iter()
                    .filter(|spec| spec.source == candidate)
                    .cloned()
                    .collect::<Vec<_>>()),
            )
        })
        .collect::<Vec<_>>();
        sources.push((source, replacement));
        Self::merge(sources)
    }
}

fn sanitize_registry_error(error: &str) -> String {
    error
        .split_whitespace()
        .map(|word| {
            let normalized = word.to_ascii_lowercase();
            if normalized.contains("token=")
                || normalized.contains("secret=")
                || normalized.contains("authorization=")
            {
                "[redacted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn command_score(spec: &CommandSpec) -> i32 {
    match spec.legacy_kind() {
        Some(TuiSlashCommandKind::Help) => 500,
        Some(TuiSlashCommandKind::Clear) => 400,
        Some(TuiSlashCommandKind::Model) => 300,
        Some(TuiSlashCommandKind::Resume) => 200,
        Some(TuiSlashCommandKind::Exit) => 100,
        Some(_) | None => 50,
    }
}

fn deduplicate_aliases(commands: &mut [CommandSpec]) {
    let names = commands.iter().map(|spec| spec.name.to_ascii_lowercase()).collect::<HashSet<_>>();
    let mut claimed = HashSet::new();
    for command in commands {
        command.aliases.retain(|alias| {
            let alias = alias.to_ascii_lowercase();
            !names.contains(&alias) && claimed.insert(alias)
        });
    }
}

fn builtin_commands() -> Vec<CommandSpec> {
    [
        (
            TuiSlashCommandKind::Help,
            "help",
            &[][..],
            "显示可用的斜杠命令",
            true,
            Some("[all]"),
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Clear,
            "clear",
            &["reset", "new"][..],
            "清空当前会话内容",
            false,
            None,
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Model,
            "model",
            &[][..],
            "从列表选择当前模型",
            false,
            None,
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Acp,
            "acp",
            &[][..],
            "从列表选择后续提交使用的 ACP 后端",
            false,
            None,
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Settings,
            "settings",
            &["config"][..],
            "打开当前会话设置面板",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Effort,
            "effort",
            &[][..],
            "设置当前会话推理强度",
            true,
            Some("[minimal|low|medium|high|xhigh]"),
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Fast,
            "fast",
            &[][..],
            "切换快速模式",
            true,
            Some("[on|off]"),
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Theme,
            "theme",
            &["color"][..],
            "预览或切换 TUI 主题",
            true,
            Some("[theme]"),
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Plan,
            "plan",
            &[][..],
            "进入只允许计划待办工具的会话模式",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Permissions,
            "permissions",
            &["allowed-tools", "fewer-permission-prompts"][..],
            "查看并回复真实待处理权限请求",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Cd,
            "cd",
            &[][..],
            "切换到经过校验并由 Gateway 解析的项目目录",
            true,
            Some("<path>"),
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Project,
            "project",
            &[][..],
            "查看当前 Gateway 项目上下文",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Mcp,
            "mcp",
            &[][..],
            "查看 Gateway MCP 连接、工具、资源与 prompts",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Memory,
            "memory",
            &[][..],
            "查看当前项目和全局记忆文件",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Skills,
            "skills",
            &[][..],
            "查看 Skills 并刷新安全动态命令",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Plugins,
            "plugins",
            &["plugin"][..],
            "查看本地插件目录",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Hooks,
            "hooks",
            &[][..],
            "查看插件提供的 Hook 能力",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Doctor,
            "doctor",
            &["checkup"][..],
            "只读检查 Gateway 集成健康状态",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::ReloadSkills,
            "reload-skills",
            &[][..],
            "刷新 Skills 与动态命令",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::ReloadPlugins,
            "reload-plugins",
            &[][..],
            "刷新插件目录",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Tasks,
            "tasks",
            &["bashes", "background", "bg"][..],
            "查看当前项目的会话任务与 Agent Scheduler 任务",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Agents,
            "agents",
            &[][..],
            "查看当前项目的 Agent Scheduler workers 与 teams",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Workflows,
            "workflows",
            &[][..],
            "查看 Workflow 应用并刷新动态执行命令",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Schedule,
            "schedule",
            &["routines", "cron"][..],
            "管理 VibeWindow Gateway Cron 任务",
            true,
            Some("[enable|disable|delete <id>]"),
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Resume,
            "resume",
            &["continue"][..],
            "恢复最近一次或指定的会话快照",
            true,
            Some("[session-id]"),
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Rename,
            "rename",
            &[][..],
            "重命名当前会话",
            true,
            Some("<title>"),
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Branch,
            "branch",
            &[][..],
            "从当前会话创建分支并切换",
            true,
            Some("[message-id]"),
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Compact,
            "compact",
            &[][..],
            "压缩当前会话上下文",
            false,
            None,
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Rewind,
            "rewind",
            &["checkpoint", "undo"][..],
            "回退到指定持久化消息",
            true,
            Some("[message-id]"),
            CommandExecutionPolicy::Queued,
        ),
        (
            TuiSlashCommandKind::Exit,
            "exit",
            &["quit"][..],
            "退出 TUI",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Status,
            "status",
            &[][..],
            "查看会话、模型和连接状态",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Context,
            "context",
            &[][..],
            "查看工作区与会话上下文",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Usage,
            "usage",
            &[][..],
            "查看真实 token 与步骤用量",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Cost,
            "cost",
            &[][..],
            "查看 provider 返回的 token 用量",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Stats,
            "stats",
            &[][..],
            "查看会话统计",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Diff,
            "diff",
            &[][..],
            "查看当前会话变更摘要",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Files,
            "files",
            &[][..],
            "查看修改文件",
            false,
            None,
            CommandExecutionPolicy::Immediate,
        ),
        (
            TuiSlashCommandKind::Export,
            "export",
            &[][..],
            "安全导出可见 transcript",
            true,
            Some("<path>"),
            CommandExecutionPolicy::Queued,
        ),
    ]
    .into_iter()
    .map(|(kind, name, aliases, summary, takes_argument, argument_hint, execution_policy)| {
        CommandSpec {
            name: name.to_string(),
            aliases: aliases.iter().map(|alias| (*alias).to_string()).collect(),
            summary: summary.to_string(),
            takes_argument,
            argument_hint: argument_hint.map(str::to_string),
            source: CommandSource::Builtin,
            availability: if kind == TuiSlashCommandKind::Fast {
                CommandAvailability::Unavailable {
                    reason: "当前 Gateway stream 契约未提供快速模式".to_string(),
                }
            } else {
                CommandAvailability::Available
            },
            execution_policy,
            action: CommandAction::Builtin(kind),
        }
    })
    .chain(std::iter::once(CommandSpec {
        name: "add-dir".to_string(),
        aliases: Vec::new(),
        summary: "添加额外文件访问目录".to_string(),
        takes_argument: true,
        argument_hint: Some("<path>".to_string()),
        source: CommandSource::Builtin,
        availability: CommandAvailability::Unavailable {
            reason: "补全目录不等于文件访问授权；当前 Gateway 未提供目录授权契约".to_string(),
        },
        execution_policy: CommandExecutionPolicy::Queued,
        action: CommandAction::Gateway("directory_authorization".to_string()),
    }))
    .chain([
        CommandSpec {
            name: "fork".to_string(),
            aliases: Vec::new(),
            summary: "创建 Claude 后台会话分支".to_string(),
            takes_argument: true,
            argument_hint: Some("[prompt]".to_string()),
            source: CommandSource::Builtin,
            availability: CommandAvailability::Unavailable {
                reason: "当前 Gateway 没有 Claude 后台会话 fork 契约；请使用 /branch 复制当前会话"
                    .to_string(),
            },
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway("background_fork".to_string()),
        },
        CommandSpec {
            name: "subtask".to_string(),
            aliases: Vec::new(),
            summary: "创建 Agent Scheduler 子任务".to_string(),
            takes_argument: true,
            argument_hint: Some("<prompt>".to_string()),
            source: CommandSource::Builtin,
            availability: CommandAvailability::Unavailable {
                reason: "创建调度任务需要 spec_uri/meta_uri 等稳定产物，不能从一句 prompt 猜造"
                    .to_string(),
            },
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway("scheduler_create_task".to_string()),
        },
        CommandSpec {
            name: "stop".to_string(),
            aliases: Vec::new(),
            summary: "停止选定的后台任务".to_string(),
            takes_argument: true,
            argument_hint: Some("<task-id>".to_string()),
            source: CommandSource::Builtin,
            availability: CommandAvailability::Unavailable {
                reason: "任务列表尚未提供可证明归属且可停止的任务，拒绝跨会话停止".to_string(),
            },
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway("scheduler_stop_task".to_string()),
        },
    ])
    .collect()
}

impl CommandSpec {
    pub(crate) fn legacy_kind(&self) -> Option<TuiSlashCommandKind> {
        match self.action {
            CommandAction::Builtin(kind) => Some(kind),
            _ => None,
        }
    }
}
