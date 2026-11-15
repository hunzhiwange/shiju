//! 安全命令处理与报告格式测试。

use super::{
    Config, guard_corpus_update_report_lines, handle_security_command,
    print_guard_corpus_update_report,
};
use cli_command_schema::SecurityCommands;
use vw_agent::security::GuardCorpusUpdateReport;

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
