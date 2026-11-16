//! Real process, SkillTool and checkpoint context ports for ACP recovery contracts.
use super::{VibeWindowBackend, backend::NativeAcpHost};
use agent_acp::host_migration_53_recovery_contracts as contracts;

struct NativeRecoveryPorts;
#[async_trait::async_trait]
impl contracts::RecoveryPorts for NativeRecoveryPorts {
    type Host = NativeAcpHost;
    async fn invoke_skill(backend: &VibeWindowBackend, id: &str, root: &std::path::Path) -> Vec<crate::skills::InvokedSkillRecord> {
        invoke_native_skill(backend, id, root).await
    }
    async fn checkpoint(root: &std::path::Path, id: &str, records: Option<Vec<crate::skills::InvokedSkillRecord>>) -> Vec<crate::skills::InvokedSkillRecord> {
        checkpoint(root, id, records).await
    }
    fn skill_body(root: &std::path::Path, sources: &super::mounts::SessionSkillSources) -> String {
        let catalog = crate::skills::load_session_skills(root, &crate::config::Config::default(), sources, true).unwrap();
        catalog.iter().find(|skill| skill.name == "recovery_probe").unwrap().prompts.join("\n")
    }
}

#[tokio::test]
async fn recovery_process_worker() {
    contracts::recovery_process_worker::<NativeRecoveryPorts>().await;
}

fn worker(root: &std::path::Path, phase: &str) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "acp::mount_recovery_tests::recovery_process_worker", "--nocapture"])
        .env("ACP_MOUNT_RECOVERY_ROOT", root)
        .env("ACP_MOUNT_RECOVERY_PHASE", phase)
        .env("VW_MODEL_RECOVERY_TEST_ROOT", root.join("home"))
        .env("VIBEWINDOW_TEST_ROOT", root.join("home"))
        .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root.join("home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "worker {phase}: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
async fn invoke_native_skill(
    backend: &VibeWindowBackend,
    id: &str,
    root: &std::path::Path,
) -> Vec<crate::skills::InvokedSkillRecord> {
    use crate::tools::{Tool, ToolUseContext, scope_tool_use_context, skill::SkillTool};
    use std::sync::Arc;
    let context = Arc::new(
        ToolUseContext::new(id, Some(root.to_str().unwrap().into()))
            .with_session_skills(backend.mounted_runtime(id).0)
            .with_explicitly_requested_skills(["recovery_probe"]),
    );
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy {
            workspace_dir: root.into(),
            workspace_only: true,
            ..Default::default()
        }),
        id.into(),
        false,
        root.into(),
        Arc::new(crate::config::Config::default()),
    )
    .with_skill_directory_context(&context);
    let result = scope_tool_use_context(
        context.clone(),
        tool.call(serde_json::json!({"skill":"recovery_probe"})),
    )
    .await
    .unwrap();
    assert!(result.is_success());
    let records = context.take_invoked_skills();
    assert_eq!(records.len(), 1);
    records
}
async fn checkpoint(
    root: &std::path::Path,
    id: &str,
    records: Option<Vec<crate::skills::InvokedSkillRecord>>,
) -> Vec<crate::skills::InvokedSkillRecord> {
    let root = root.canonicalize().unwrap();
    let id = id.to_string();
    crate::project::instance::provide(root, None, move || {
        Box::pin(async move {
            crate::session::ui_store::with_tenant_scope(Default::default(), async move {
                let history =
                    vec![serde_json::json!({"role":"user","content":"invoke native skill"})];
                if let Some(records) = records {
                    crate::session::skill_checkpoint::save(&id, "main", &history, &records)
                        .await
                        .unwrap();
                }
                crate::session::skill_checkpoint::restore(&id, "main", &history).await.unwrap()
            })
            .await
        })
    })
    .await
    .unwrap()
}

#[test]
fn acp_mount_recovery_independent_processes_restore_and_clear() {
    contracts::acp_mount_recovery_independent_processes_restore_and_clear(worker);
}

#[test]
fn acp_mount_recovery_replaced_source_rejected() {
    contracts::acp_mount_recovery_replaced_source_rejected(worker);
}

#[test]
fn acp_mount_recovery_corrupt_and_foreign_records_rejected() {
    contracts::acp_mount_recovery_corrupt_and_foreign_records_rejected(worker);
}

#[test]
fn acp_mount_recovery_persistence_failure_retains_old_revision() {
    contracts::acp_mount_recovery_persistence_failure_retains_old_revision(worker);
}

#[test]
fn acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk() {
    contracts::acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk(worker);
}

#[test]
fn acp_mount_recovery_explicit_load_replaces_and_clears() {
    contracts::acp_mount_recovery_explicit_load_replaces_and_clears(worker);
}

#[test]
fn acp_mount_recovery_revoked_and_edited_sources_rejected() {
    contracts::acp_mount_recovery_revoked_and_edited_sources_rejected(worker);
}

#[test]
fn acp_mount_recovery_committed_revision_survives_exit_before_publication() {
    contracts::acp_mount_recovery_committed_revision_survives_exit_before_publication(worker);
}
