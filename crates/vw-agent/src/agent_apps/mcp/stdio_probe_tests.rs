//! Real local stdio protocol and process-tree evidence for diagnostic handshakes.
use super::{
    catalog_create_tests::{http_draft, with_database},
    network_probe_tests::{Fixture, TOKEN},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::time::timeout;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticFailureCode, DiagnosticJobState, DiagnosticOutcome, DiagnosticResult,
    },
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

#[test]
fn diagnostic_mcp_stdio_cancellation_reaps_process_tree_handshake() -> anyhow::Result<()> {
    with_database(|db| {
        let fixture = Fixture::new(db)?;
        let workdir = fixture.root.path().join("stdio");
        std::fs::create_dir(&workdir)?;
        let mut records = Vec::new();
        let mut permits = Vec::new();
        for mode in ["normal", "cancel", "timeout", "missing", "credential"] {
            let mut draft = http_draft(mode, "unused");
            draft.connection = McpConnectionDefinition::Stdio {
                command: if mode == "missing" { "/nonexistent/shiju-mcp-probe" } else { "/bin/sh" }
                    .into(),
                args: vec!["-c".into(), STDIO_HANDSHAKE.into(), "fixture".into(), mode.into()],
                env: [
                    ("PROBE_ENV".into(), McpConfigValue::Literal { value: "authorized".into() }),
                    ("PATH".into(), McpConfigValue::Literal { value: "/authorized/only".into() }),
                ]
                .into(),
                cwd: Some("stdio".into()),
            };
            if mode == "credential" {
                let McpConnectionDefinition::Stdio { env, .. } = &mut draft.connection else {
                    unreachable!()
                };
                env.insert(
                    "TOKEN".into(),
                    McpConfigValue::Credential {
                        reference: vw_api_types::mcp_management::McpCredentialRef {
                            service: vw_api_types::agent_apps_auth::AuthService::CodexOauth,
                            account_id: "ungranted-account".into(),
                        },
                    },
                );
            }
            let digest = hex::encode(Sha256::digest(serde_json::to_vec(&draft.connection)?));
            let record = fixture.catalog.create(&fixture.catalog_scope(), draft).unwrap();
            permits.push(json!({
                "project_id":"project", "app_id":AppId::Claude, "record_id":record.record.id,
                "connection_revision":record.connection_revision,
                "stdio_sha256":digest, "stdio_workspace":fixture.root.path(),
            }));
            records.push(record);
        }
        fixture.policy(&[])?;
        assert!(fixture.start(TOKEN, fixture.owner.scope.clone(), &[&records[0]]).is_err());
        assert!(!workdir.join("normal.pid").exists());
        let mut wrong = permits.clone();
        wrong[0]["stdio_sha256"] = json!("0".repeat(64));
        fixture.policy(&wrong)?;
        assert!(fixture.start(TOKEN, fixture.owner.scope.clone(), &[&records[0]]).is_err());
        assert!(!workdir.join("normal.pid").exists());
        fixture.policy(&permits)?;
        assert!(fixture.start(TOKEN, fixture.owner.scope.clone(), &[&records[4]]).is_err());
        assert!(!workdir.join("credential.pid").exists());
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            for (record, mode) in records.iter().zip(["normal", "cancel", "timeout", "missing"]) {
                let job = fixture.start(TOKEN, fixture.owner.scope.clone(), &[record]);
                assert!(job.is_ok(), "authorized stdio handshake was rejected: {job:?}");
                let job = job.unwrap();
                if mode == "missing" {
                    let snapshot = fixture.finish(&job).await?;
                    assert!(matches!(&snapshot.results.unwrap()[0], DiagnosticResult::Mcp {
                        report, ..
                    } if matches!(report.outcome, DiagnosticOutcome::Failed { .. })));
                    continue;
                }
                let (pid, child) = timeout(Duration::from_secs(10), async {
                    loop {
                        let read = |suffix| {
                            std::fs::read_to_string(workdir.join(format!("{mode}.{suffix}")))
                                .ok()
                                .and_then(|s| s.trim().parse::<i32>().ok())
                        };
                        if let (Some(pid), Some(child)) = (read("pid"), read("child")) {
                            if workdir
                                .join(format!("{mode}.requests"))
                                .metadata()
                                .is_ok_and(|m| m.len() > 0)
                            {
                                break (pid, child);
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await?;
                if mode != "normal" {
                    // SAFETY: signal zero only observes these fixture process IDs.
                    assert!(unsafe { libc::kill(pid, 0) == 0 && libc::kill(child, 0) == 0 });
                    eprintln!("controlled stdio {mode}: leader={pid}, child={child}, both alive");
                }
                if mode == "cancel" {
                    fixture.jobs.cancel(&fixture.owner, &job.task_id).unwrap();
                } else if mode == "timeout" {
                    tokio::time::pause();
                    tokio::time::advance(Duration::from_secs(31)).await;
                    tokio::time::resume();
                }
                let snapshot = fixture.finish(&job).await?;
                if mode == "cancel" {
                    assert_eq!(snapshot.state, DiagnosticJobState::Cancelled {});
                } else {
                    let results = snapshot.results.unwrap();
                    let DiagnosticResult::Mcp { report, .. } = &results[0] else {
                        panic!("wrong result")
                    };
                    if mode == "normal" {
                        assert!(matches!(report.outcome, DiagnosticOutcome::Succeeded { .. }));
                    } else {
                        assert!(
                            matches!(report.outcome, DiagnosticOutcome::Failed { ref error, .. }
                            if error.code == DiagnosticFailureCode::Timeout)
                        );
                    }
                }
                timeout(Duration::from_secs(5), async {
                    // Observe real OS process exit, including the launcher's child.
                    while unsafe { libc::kill(pid, 0) == 0 || libc::kill(child, 0) == 0 } {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await?;
                eprintln!("controlled stdio {mode}: leader={pid}, child={child}, both exited");
                assert!(
                    !std::fs::read_to_string(workdir.join(format!("{mode}.requests")))?
                        .contains("tools/list")
                );
            }
            Ok(())
        })
    })
}

const STDIO_HANDSHAKE: &str = r#"
    [ "$PROBE_ENV" = authorized ] && [ "$PATH" = /authorized/only ] &&
    [ "${PWD##*/}" = stdio ] || exit 1
    printf '%s\n' "$$" > "$1.pid"
    /bin/sleep 120 &
    printf '%s\n' "$!" > "$1.child"
    while IFS= read -r line; do
        printf '%s\n' "$line" >> "$1.requests"
        case "$line" in
            *'"method":"initialize"'*)
                if [ "$1" != normal ]; then wait; fi
                id=${line#*\"id\":}
                id=${id%%,*}
                id=${id%%\}*}
                printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"fixture","version":"1"}}}\n' "$id"
                ;;
        esac
    done
    wait
"#;
