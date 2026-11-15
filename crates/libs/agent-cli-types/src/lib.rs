//! CLI 命令数据契约。命令执行由调用方负责。

mod cli_commands;

pub use cli_commands::{
    ChannelCommands, CronCommands, IntegrationCommands, MemoryCommands, MigrateCommands,
    PluginCommands, PluginMarketplaceCommands, ServiceCommands, SkillCommands,
};

#[cfg(test)]
mod cli_commands_tests;
