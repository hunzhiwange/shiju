//! 远程 Gateway API CLI 的共享边界。

use std::io::{Read, Write};

use anyhow::Result;
use clap::{ArgMatches, Args, Command, Error, FromArgMatches, Subcommand};

use cli_api_system::config::{self, ConfigCommands};
use cli_api_core::context::ApiContext;
pub use cli_api_core::ApiOptions;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
use cli_api_core::ensure_confirmed;
#[cfg(any(test, target_arch = "wasm32"))]
use cli_api_core::{ApiOutputFormat, ensure_confirmed};
use cli_api_system::control::{self, ControlCommands, ProviderCommands};
use cli_api_system::desktop::{self, DesktopCommands};
use cli_api_system::extensions::{self, ExtensionCommands};
use cli_api_session::interactive::{self, InteractiveCommands};
use cli_api_workspace::knowledge::{self, KnowledgeCommands};
use cli_api_system::operations::{self, OperationsCommands};
use cli_api_workspace::project::{self, ProjectCommands};
use cli_api_session::realtime::{self, RealtimeCommands};
use cli_api_automation::scheduler::{self, SchedulerCommands};
use cli_api_session::session::{self, SessionCommands};
use cli_api_session::session_content::{self, SessionContentCommands};
use cli_api_system::shiju_apps::{self, ShijuAppsCommands};
use cli_api_system::skill_market::{self, SkillMarketCommands};
use cli_api_automation::task_lifecycle::{self, TaskLifecycleCommands};
use cli_api_automation::task_pool::{self, TaskPoolCommands};
use cli_api_automation::workflow::{self, WorkflowCommands};
use cli_api_workspace::workspace::{self, WorkspaceCommands};

use cli_api_system::system::{self, SystemCommands};

/// 一次根级 Gateway API 命令调用的完整参数。
#[derive(Debug)]
pub struct ApiArgs {
    options: ApiOptions,
    command: ApiCommands,
}

impl ApiArgs {
    fn new(options: ApiOptions, command: ApiCommands) -> Self {
        Self { options, command }
    }
}

/// API 领域命令注册表。
#[derive(Subcommand, Debug)]
enum ApiCommands {
    /// 读取或受控更新 Gateway 配置。
    Config {
        /// 配置读取与更新操作。
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// 查询 Gateway 可见的 provider。
    Provider {
        /// Provider 操作。
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// 管理认证、plugin hooks 与 Gateway 实例。
    Control {
        /// 受控副作用操作。
        #[command(subcommand)]
        command: ControlCommands,
    },
    /// 管理 Cron、集成、记忆与诊断接口。
    Operations {
        /// 远程运维操作。
        #[command(subcommand)]
        command: OperationsCommands,
    },
    /// 处理权限、问题与 PTY 交互。
    Interactive {
        /// 远程交互操作。
        #[command(subcommand)]
        command: InteractiveCommands,
    },
    /// 管理远程项目与 worktree 生命周期。
    Project {
        /// 项目与 worktree 操作。
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// 执行推理并消费 Gateway 实时协议。
    Realtime {
        /// 推理、SSE、stop 与 WebSocket 操作。
        #[command(subcommand)]
        command: RealtimeCommands,
    },
    /// 管理知识库资源、索引与检索。
    Knowledge {
        /// 知识库操作。
        #[command(subcommand)]
        command: KnowledgeCommands,
    },
    /// 运维 Task Pool worktree 池。
    TaskPool {
        /// Task Pool 操作。
        #[command(subcommand)]
        command: TaskPoolCommands,
    },
    /// 管理任务生命周期 Workflow 定义与状态。
    TaskLifecycle {
        /// 任务生命周期操作。
        #[command(subcommand)]
        command: TaskLifecycleCommands,
    },
    /// 管理 Agent Scheduler 项目任务、Worker、Team 与运维队列。
    Scheduler {
        /// Scheduler 项目操作。
        #[command(subcommand)]
        command: SchedulerCommands,
    },
    /// 管理 Session 核心资源与生命周期。
    Session {
        /// Session 核心操作。
        #[command(subcommand)]
        command: SessionCommands,
    },
    /// 管理 Session 消息内容、UI 快照与状态。
    SessionContent {
        /// Session 内容与 UI 操作。
        #[command(subcommand)]
        command: SessionContentCommands,
    },
    /// 访问 SHIJU 账号与应用接口。
    ShijuApps {
        /// SHIJU 账号与应用操作。
        #[command(subcommand)]
        command: ShijuAppsCommands,
    },
    /// 发现 SHIJU 与统一市场技能。
    SkillMarket {
        /// 技能市场操作。
        #[command(subcommand)]
        command: SkillMarketCommands,
    },
    /// 管理 Gateway 本地 skills 与 plugins。
    Extensions {
        /// 扩展管理操作。
        #[command(subcommand)]
        command: ExtensionCommands,
    },
    /// 访问远程 Gateway 主机的 desktop 专用接口。
    Desktop {
        /// Desktop 偏好、技能与宿主动作。
        #[command(subcommand)]
        command: DesktopCommands,
    },
    /// 管理 Workflow 应用、恢复运行与聊天流。
    Workflow {
        /// Workflow 应用操作。
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// 搜索、读取并受控修改远程工作区。
    Workspace {
        /// 文件、搜索与 Git 操作。
        #[command(subcommand)]
        command: WorkspaceCommands,
    },
    /// 查询 Gateway 系统状态与发现信息。
    System {
        /// 系统状态与发现操作。
        #[command(subcommand)]
        command: SystemCommands,
    },
}

/// 将 API 命令族直接注册到 CLI 根级，同时给每个命令族附加共享参数。
#[derive(Debug)]
pub struct RootApiCommands(ApiArgs);

impl RootApiCommands {
    pub fn into_args(self) -> ApiArgs {
        self.0
    }
}

impl FromArgMatches for RootApiCommands {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        let command = ApiCommands::from_arg_matches(matches)?;
        let subcommand_matches = matches.subcommand().map_or(matches, |(_, matches)| matches);
        let options = ApiOptions::from_arg_matches(subcommand_matches)?;
        Ok(Self(ApiArgs::new(options, command)))
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Subcommand for RootApiCommands {
    fn augment_subcommands(command: Command) -> Command {
        augment_api_subcommands(command, false)
    }

    fn augment_subcommands_for_update(command: Command) -> Command {
        augment_api_subcommands(command, true)
    }

    fn has_subcommand(name: &str) -> bool {
        name != "config" && ApiCommands::has_subcommand(name)
    }
}

fn augment_api_subcommands(command: Command, for_update: bool) -> Command {
    let api_root = if for_update {
        ApiCommands::augment_subcommands_for_update(Command::new("api"))
    } else {
        ApiCommands::augment_subcommands(Command::new("api"))
    };
    let subcommands = api_root
        .get_subcommands()
        .filter(|subcommand| subcommand.get_name() != "config")
        .cloned()
        .map(|subcommand| {
            if for_update {
                ApiOptions::augment_args_for_update(subcommand)
            } else {
                ApiOptions::augment_args(subcommand)
            }
        })
        .collect::<Vec<_>>();
    command.subcommands(subcommands)
}

/// 根级 `config` 中的远程 Gateway 配置子命令。
#[derive(Debug)]
pub struct RootConfigCommands(config::ConfigCommands);

impl FromArgMatches for RootConfigCommands {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        config::ConfigCommands::from_arg_matches(matches).map(Self)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        self.0.update_from_arg_matches(matches)
    }
}

impl Subcommand for RootConfigCommands {
    fn augment_subcommands(command: Command) -> Command {
        config::ConfigCommands::augment_subcommands(command)
    }

    fn augment_subcommands_for_update(command: Command) -> Command {
        config::ConfigCommands::augment_subcommands_for_update(command)
    }

    fn has_subcommand(name: &str) -> bool {
        config::ConfigCommands::has_subcommand(name)
    }
}

pub fn config_args(options: ApiOptions, command: RootConfigCommands) -> ApiArgs {
    ApiArgs::new(options, ApiCommands::Config { command: command.0 })
}

impl ApiCommands {
    fn dangerous_operation(
        &self,
        directory: Option<&str>,
        platform: &str,
        company: &str,
        host: &str,
        port: u16,
    ) -> Option<String> {
        match self {
            Self::Config { command } => command.dangerous_operation(directory, platform, company),
            Self::Control { command } => command.dangerous_operation(directory, platform, company),
            Self::Operations { command } => {
                command.dangerous_operation(directory, platform, company)
            }
            Self::Interactive { command } => command.dangerous_operation(),
            Self::Project { command } => command.dangerous_operation(),
            Self::Realtime { command } => command.dangerous_operation(),
            Self::Knowledge { command } => command.dangerous_operation(),
            Self::TaskPool { command } => command.dangerous_operation(),
            Self::TaskLifecycle { command } => command.dangerous_operation(),
            Self::Scheduler { command } => command.dangerous_operation(),
            Self::Session { command } => command.dangerous_operation(),
            Self::SessionContent { command } => command.dangerous_operation(),
            Self::ShijuApps { command } => command.dangerous_operation(),
            Self::SkillMarket { command } => command.dangerous_operation(),
            Self::Extensions { command } => command.dangerous_operation(),
            Self::Desktop { command } => command.dangerous_operation(host, port),
            Self::Workflow { command } => command.dangerous_operation(),
            Self::Provider { .. } | Self::System { .. } | Self::Workspace { .. } => None,
        }
    }
}

/// 执行远程 Gateway API 命令。
pub async fn run(args: ApiArgs) -> Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    run_with_io(args, &mut stdin, &mut stdout).await
}

#[cfg(test)]
async fn run_with_writer<W: Write>(args: ApiArgs, writer: &mut W) -> Result<()> {
    let mut stdin = std::io::empty();
    run_with_io(args, &mut stdin, writer).await
}

async fn run_with_io<R: Read, W: Write>(
    args: ApiArgs,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let yes = args.options.yes();
    if let Some(operation) = args.command.dangerous_operation(
        args.options.directory(),
        args.options.platform(),
        args.options.company(),
        args.options.host(),
        args.options.port(),
    ) {
        ensure_confirmed(args.options.yes(), &operation)?;
    }

    let context = ApiContext::from_options(&args.options)?;
    match args.command {
        ApiCommands::Config { command } => config::run(command, &context, stdin, writer).await,
        ApiCommands::Provider { command } => {
            control::run_provider(command, &context, stdin, writer).await
        }
        ApiCommands::Control { command } => control::run(command, &context, stdin, writer).await,
        ApiCommands::Operations { command } => {
            operations::run(command, &context, stdin, writer).await
        }
        ApiCommands::Interactive { command } => {
            interactive::run(command, &context, stdin, writer).await
        }
        ApiCommands::Project { command } => {
            project::run(command, &context, yes, stdin, writer).await
        }
        ApiCommands::Realtime { command } => realtime::run(command, &context, stdin, writer).await,
        ApiCommands::Knowledge { command } => {
            knowledge::run(command, &context, stdin, writer).await
        }
        ApiCommands::TaskPool { command } => task_pool::run(command, &context, stdin, writer).await,
        ApiCommands::TaskLifecycle { command } => {
            task_lifecycle::run(command, &context, stdin, writer).await
        }
        ApiCommands::Scheduler { command } => {
            scheduler::run(command, &context, stdin, writer).await
        }
        ApiCommands::Session { command } => session::run(command, &context, stdin, writer).await,
        ApiCommands::SessionContent { command } => {
            session_content::run(command, &context, stdin, writer).await
        }
        ApiCommands::ShijuApps { command } => {
            shiju_apps::run(command, &context, stdin, writer).await
        }
        ApiCommands::SkillMarket { command } => {
            skill_market::run(command, &context, stdin, writer).await
        }
        ApiCommands::Extensions { command } => extensions::run(command, &context, writer).await,
        ApiCommands::Desktop { command } => desktop::run(command, &context, stdin, writer).await,
        ApiCommands::Workflow { command } => workflow::run(command, &context, stdin, writer).await,
        ApiCommands::System { command } => system::run(command, &context, writer).await,
        ApiCommands::Workspace { command } => {
            workspace::run(command, &context, yes, stdin, writer).await
        }
    }
}

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod control_tests;
#[cfg(test)]
mod desktop_tests;
#[cfg(test)]
mod extensions_tests;
#[cfg(test)]
mod interactive_tests;
#[cfg(test)]
mod knowledge_tests;
#[cfg(test)]
mod operations_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod realtime_tests;
#[cfg(test)]
mod scheduler_tests;
#[cfg(test)]
mod session_content_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod shiju_apps_tests;
#[cfg(test)]
mod skill_market_tests;
#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod task_lifecycle_tests;
#[cfg(test)]
mod task_pool_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workflow_tests;
#[cfg(test)]
mod workspace_tests;

#[cfg(test)]
mod test_cli;
