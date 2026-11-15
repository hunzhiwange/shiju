//! 仅为库内私有行为测试组合远程命令；完整 CLI 集成测试保留在 vw-cli。

use clap::{Parser, Subcommand};
use super::{ApiArgs, ApiOptions, RootApiCommands, RootConfigCommands, config_args};

#[derive(Debug, Parser)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    #[command(flatten)]
    Api(RootApiCommands),
    Config {
        #[command(flatten)]
        api_options: ApiOptions,
        #[command(subcommand)]
        command: RootConfigCommands,
    },
}

impl Commands {
    pub(super) fn into_api_args(self) -> Option<ApiArgs> {
        match self {
            Self::Api(command) => Some(command.into_args()),
            Self::Config { api_options, command } => Some(config_args(api_options, command)),
        }
    }
}
