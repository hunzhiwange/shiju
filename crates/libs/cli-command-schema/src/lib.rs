//! Command argument definitions and shell completion generation.

mod cli;
mod temperature;

pub use cli::{
    Cli, Commands, CompletionShell, ConfigCommands, DoctorCommands, EmployeeAcpKind, EstopLevelArg,
    EstopSubcommands, SecurityCommands, TaskCommands, write_shell_completion,
};
use temperature::parse_temperature;

#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod cli_workbench_parse_tests;
#[cfg(test)]
mod temperature_lib_tests;
#[cfg(test)]
mod temperature_tests;
