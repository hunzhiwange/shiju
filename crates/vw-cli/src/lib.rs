//! CLI 二进制的共享启动入口。

pub use cli_bootstrap::run;

#[cfg(test)]
use cli_command_schema as cli;
#[cfg(test)]
mod handlers;
#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
