//! CLI 装配层的 ACP 命令契约测试。

use clap::Parser;

#[test]
fn acp_command_has_no_command_specific_flags() {
    let parsed = super::cli::Cli::try_parse_from(["vibewindow", "acp"]).expect("ACP command");
    assert!(matches!(parsed.command, super::cli::Commands::Acp));
    assert!(super::cli::Cli::try_parse_from(["vibewindow", "acp", "--listen"]).is_err());
}
