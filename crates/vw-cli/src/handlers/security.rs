//! 处理 CLI 安全相关命令。
//! 模块把安全策略展示与用户确认流程限制在 CLI 边界内。

use anyhow::Result;

use crate::cli::SecurityCommands;
use crate::config::Config;
use vw_agent::security;

fn guard_corpus_update_report_lines(report: &security::GuardCorpusUpdateReport) -> [String; 6] {
    [
        "Semantic guard corpus update completed.".to_string(),
        format!("  Source:           {}", report.source),
        format!("  SHA-256:          {}", report.sha256),
        format!("  Parsed records:   {}", report.parsed_records),
        format!("  Upserted records: {}", report.upserted_records),
        format!("  Collection:       {}", report.collection),
    ]
}

fn print_guard_corpus_update_report(report: &security::GuardCorpusUpdateReport) {
    for line in guard_corpus_update_report_lines(report) {
        println!("{line}");
    }
}

/// 执行 handle_security_command 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) async fn handle_security_command(
    config: &Config,
    security_command: SecurityCommands,
) -> Result<()> {
    match security_command {
        SecurityCommands::UpdateGuardCorpus { source, checksum } => {
            let report = security::semantic_guard::update_guard_corpus(
                config,
                source.as_deref(),
                checksum.as_deref(),
            )
            .await?;

            print_guard_corpus_update_report(&report);
            Ok(())
        }
    }
}
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
