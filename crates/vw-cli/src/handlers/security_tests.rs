//! `security` 子命令的注册、解析与失败边界测试。
//! 校验防护语料更新参数、报告行格式和完整输出。
//! 校验和不匹配、本地源不可读或集合为空时，必须在访问后端前拒绝请求。

use clap::{CommandFactory, Parser};

use super::{
    Config, guard_corpus_update_report_lines, handle_security_command,
    print_guard_corpus_update_report,
};
use crate::cli::{Cli, Commands, SecurityCommands};
use vw_agent::security::GuardCorpusUpdateReport;

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

#[test]
fn guard_corpus_update_report_lines_include_labels_and_values() {
    let report = GuardCorpusUpdateReport {
        source: "builtin".to_string(),
        sha256: "abc123".to_string(),
        parsed_records: 7,
        upserted_records: 5,
        collection: "guard-corpus".to_string(),
    };

    assert_eq!(
        guard_corpus_update_report_lines(&report),
        [
            "Semantic guard corpus update completed.".to_string(),
            "  Source:           builtin".to_string(),
            "  SHA-256:          abc123".to_string(),
            "  Parsed records:   7".to_string(),
            "  Upserted records: 5".to_string(),
            "  Collection:       guard-corpus".to_string(),
        ]
    );
}

#[test]
fn print_guard_corpus_update_report_emits_each_report_line() {
    let report = GuardCorpusUpdateReport {
        source: "builtin".to_string(),
        sha256: "abc123".to_string(),
        parsed_records: 7,
        upserted_records: 5,
        collection: "guard-corpus".to_string(),
    };

    print_guard_corpus_update_report(&report);
}

#[tokio::test]
async fn handle_security_command_rejects_checksum_mismatch_before_backend_access() {
    let error = handle_security_command(
        &Config::default(),
        SecurityCommands::UpdateGuardCorpus {
            source: Some(" builtin ".to_string()),
            checksum: Some("deadbeef".to_string()),
        },
    )
    .await
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("guard corpus checksum mismatch"));
    assert!(message.contains("expected deadbeef"));
}

#[tokio::test]
async fn handle_security_command_reports_unreadable_local_source_before_backend_access() {
    let source = std::env::temp_dir().to_string_lossy().into_owned();
    let error = handle_security_command(
        &Config::default(),
        SecurityCommands::UpdateGuardCorpus { source: Some(source), checksum: None },
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("failed to read guard corpus file at"));
}

#[tokio::test]
async fn handle_security_command_rejects_empty_collection_before_upsert() {
    let mut config = Config::default();
    config.security.semantic_guard_collection = "   ".to_string();

    let error = handle_security_command(
        &config,
        SecurityCommands::UpdateGuardCorpus { source: None, checksum: None },
    )
    .await
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("semantic guard corpus update unavailable"));
    assert!(message.contains("security.semantic_guard_collection is empty"));
}
