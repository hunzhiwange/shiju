use clap::Parser;

use crate::{Cli, Commands};

#[test]
fn cli_workbench_parse_accepts_skill_workflow() {
    for args in [
        vec!["list"],
        vec!["start", "acp", "acp-claude", "--yes"],
        vec!["send", "session-a", "--request-id", "review-01", "--yes"],
        vec!["wait", "session-a", "--request-id", "review-01", "--timeout-secs", "60"],
        vec!["status", "session-a", "--request-id", "review-01"],
        vec!["read", "session-a", "--request-id", "review-01", "--output", "jsonl"],
        vec![
            "--host",
            "127.0.0.1",
            "--port",
            "3000",
            "--platform",
            "local",
            "--company",
            "local",
            "--user",
            "local-user",
            "--skey-env",
            "GATEWAY_SKEY",
            "list",
            "--project",
            "demo",
        ],
    ] {
        let cli = Cli::try_parse_from(["shiju", "workbench"].into_iter().chain(args)).unwrap();
        assert!(matches!(cli.command, Commands::Workbench(_)));
    }
}

#[test]
fn cli_workbench_parse_requires_delivery_id_and_bounded_wait() {
    for args in [
        vec!["start", "acp"],
        vec!["send", "session-a", "--yes"],
        vec!["status", "session-a"],
        vec!["read", "session-a"],
        vec!["wait", "session-a", "--request-id", "review-01", "--timeout-secs", "0"],
        vec!["wait", "session-a", "--request-id", "review-01", "--timeout-secs", "3601"],
    ] {
        assert!(Cli::try_parse_from(["shiju", "workbench"].into_iter().chain(args)).is_err());
    }
}
