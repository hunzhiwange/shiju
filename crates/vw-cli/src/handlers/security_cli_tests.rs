//! 安全命令在 CLI 根定义中的注册与解析测试。

use clap::{CommandFactory, Parser};
use crate::cli::{Cli, Commands, SecurityCommands};

#[test]
fn security_command_is_wired_into_cli_definition() {
    let command = Cli::command();
    let security = command
        .get_subcommands()
        .find(|command| command.get_name() == "security")
        .expect("security subcommand should be registered");

    assert!(security.get_subcommands().any(|command| command.get_name() == "update-guard-corpus"));
}

#[test]
fn update_guard_corpus_parses_optional_source_and_checksum() {
    let cli = Cli::parse_from([
        "vibewindow",
        "security",
        "update-guard-corpus",
        "--source",
        "builtin",
        "--checksum",
        "abc123",
    ]);

    let Commands::Security { security_command } = cli.command else {
        panic!("security command expected");
    };
    let SecurityCommands::UpdateGuardCorpus { source, checksum } = security_command;

    assert_eq!(source.as_deref(), Some("builtin"));
    assert_eq!(checksum.as_deref(), Some("abc123"));
}

#[test]
fn update_guard_corpus_allows_omitted_source_and_checksum() {
    let cli = Cli::parse_from(["vibewindow", "security", "update-guard-corpus"]);

    let Commands::Security { security_command } = cli.command else {
        panic!("security command expected");
    };
    let SecurityCommands::UpdateGuardCorpus { source, checksum } = security_command;

    assert_eq!(source, None);
    assert_eq!(checksum, None);
}
