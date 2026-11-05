//! Restart the production backend in independent processes with isolated storage.
use super::VibeWindowBackend;
use vw_acp::server::{BackendMountRequest, SessionBackend};
use vw_acp::{
    SessionMountRevision, SessionMountSelection, SessionSkillMount, SetSessionMountsRequest,
};

fn request(source: &std::path::Path) -> BackendMountRequest {
    BackendMountRequest {
        skills: Some(SessionMountSelection {
            version: 1,
            skills: vec![SessionSkillMount {
                name: "recovery_probe".into(),
                path: source.to_str().unwrap().into(),
            }],
        }),
        mcp_servers: vec![],
    }
}

#[tokio::test]
async fn recovery_process_worker() {
    let Some(root) = std::env::var_os("ACP_MOUNT_RECOVERY_ROOT") else { return };
    let root = std::path::PathBuf::from(root);
    let backend = if phase_is_foreign() {
        VibeWindowBackend::new(
            vw_api_types::id::TenantScope::new("foreign-platform", "foreign-company").unwrap(),
        )
    } else {
        VibeWindowBackend::default()
    };
    let phase = std::env::var("ACP_MOUNT_RECOVERY_PHASE").unwrap();
    if phase == "create" {
        let mut initial = request(&root.join("source"));
        initial.mcp_servers = vec![mcp(&root)];
        let created = backend.create_session_with_mounts(root.clone(), initial).await.unwrap();
        let id = created.session.id;
        backend
            .set_session_mounts(SetSessionMountsRequest {
                session_id: id.clone().into(),
                expected_revision: SessionMountRevision(0),
                skills: request(&root.join("source")).skills.unwrap().skills,
                mcp_servers: vec![mcp(&root)],
            })
            .await
            .unwrap();
        assert_native_call(&backend, &id).await;
        let records = invoke_native_skill(&backend, &id, &root).await;
        checkpoint(&root, &id, Some(records)).await;
        std::fs::write(root.join("session"), id).unwrap();
        return;
    }
    let id = std::fs::read_to_string(root.join("session")).unwrap();
    let loaded = backend
        .load_session_with_mounts(
            &id,
            root.clone(),
            BackendMountRequest {
                skills: match phase.as_str() {
                    "explicit_clear" => Some(SessionMountSelection { version: 1, skills: vec![] }),
                    "replace" => request(&root.join("replacement")).skills,
                    _ => None,
                },
                mcp_servers: if phase == "restore" || phase == "rollback" {
                    vec![mcp(&root)]
                } else {
                    vec![]
                },
            },
        )
        .await;
    if phase == "reject" || phase == "foreign" || phase == "corrupt" {
        assert!(loaded.is_err(), "invalid recovery must fail");
        return;
    }
    let loaded = loaded.unwrap();
    assert_eq!(
        loaded.mounts.revision,
        SessionMountRevision(
            if ["empty", "explicit_clear", "replace", "replaced_restore"].contains(&phase.as_str())
            {
                2
            } else {
                1
            }
        )
    );
    if phase == "empty" || phase == "explicit_clear" {
        assert!(loaded.mounts.skills.is_empty());
        assert!(checkpoint(&root, &id, None).await.is_empty());
        return;
    }
    assert_eq!(loaded.mounts.skills, ["recovery_probe"]);
    let (sources, _) = backend.mounted_runtime(&id);
    sources.validate().unwrap();
    invoke_native_skill(&backend, &id, &root).await;
    if phase == "replace" || phase == "replaced_restore" {
        assert!(checkpoint(&root, &id, None).await.is_empty());
    }
    if phase == "restore" {
        assert!(!checkpoint(&root, &id, None).await.is_empty());
    }
    if phase == "crash_commit" {
        let scope =
            crate::session::runtime::SessionScope::new(Default::default(), root.clone()).unwrap();
        let prepared = super::mounts::prepare_revision(
            &scope,
            &id,
            BackendMountRequest {
                skills: Some(SessionMountSelection { version: 1, skills: vec![] }),
                mcp_servers: vec![],
            },
            &[],
            SessionMountRevision(2),
        )
        .await
        .unwrap();
        super::mount_store::save(&scope, &id, &prepared).await.unwrap();
        // Simulate process death at the disk commit point, with old memory still bound.
        assert_eq!(backend.mounted_runtime(&id).0.mounts().len(), 1);
        std::process::exit(0);
    }
    let catalog = crate::skills::load_session_skills(
        &root,
        &crate::config::Config::default(),
        &sources,
        true,
    )
    .unwrap();
    let skill = catalog.iter().find(|skill| skill.name == "recovery_probe").unwrap();
    assert!(skill.prompts.join("\n").contains(
        if phase == "replace" || phase == "replaced_restore" {
            "Replacement native source"
        } else {
            "Read native sources"
        }
    ));
    if phase == "restore" {
        assert_native_call(&backend, &id).await;
    }
    if phase == "no_mcp" {
        assert!(backend.mounted_runtime(&id).1.is_empty());
    }
    if phase == "rollback" {
        let record = record_path(&root);
        let backup = record.with_extension("backup");
        std::fs::rename(&record, &backup).unwrap();
        std::fs::create_dir(&record).unwrap();
        let result = backend
            .set_session_mounts(SetSessionMountsRequest {
                session_id: id.clone().into(),
                expected_revision: SessionMountRevision(1),
                skills: vec![],
                mcp_servers: vec![],
            })
            .await;
        assert!(result.unwrap_err().to_string().contains("persist"));
        sources.validate().unwrap();
        assert_native_call(&backend, &id).await;
        assert_eq!(backend.mounted_runtime(&id).0.mounts().len(), 1);
        std::fs::remove_dir(&record).unwrap();
        std::fs::rename(backup, record).unwrap();
        assert!(!checkpoint(&root, &id, None).await.is_empty());
        let restored = backend
            .load_session_with_mounts(
                &id,
                root.clone(),
                BackendMountRequest { skills: None, mcp_servers: vec![] },
            )
            .await
            .unwrap();
        assert_eq!(restored.mounts.revision, SessionMountRevision(1));
    }
    if phase == "clear" {
        backend
            .set_session_mounts(SetSessionMountsRequest {
                session_id: id.into(),
                expected_revision: SessionMountRevision(1),
                skills: vec![],
                mcp_servers: vec![],
            })
            .await
            .unwrap();
    }
}

fn worker(root: &std::path::Path, phase: &str) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "acp::mount_recovery_tests::recovery_process_worker", "--nocapture"])
        .env("ACP_MOUNT_RECOVERY_ROOT", root)
        .env("ACP_MOUNT_RECOVERY_PHASE", phase)
        .env("VW_MODEL_RECOVERY_TEST_ROOT", root.join("home"))
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
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("attachment.txt"), "native disk attachment").unwrap();
    std::fs::create_dir(root.path().join("home")).unwrap();
    std::fs::create_dir(root.path().join("source")).unwrap();
    std::fs::write(
        root.path().join("source/SKILL.md"),
        "---\nname: recovery_probe\ndescription: recovery test\n---\nRead native sources.\n",
    )
    .unwrap();
    root
}
#[test]
fn acp_mount_recovery_independent_processes_restore_and_clear() {
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "restore");
    worker(root.path(), "clear");
    worker(root.path(), "empty");
}
#[test]
fn acp_mount_recovery_replaced_source_rejected() {
    let root = fixture();
    worker(root.path(), "create");
    std::fs::rename(root.path().join("source"), root.path().join("old-source")).unwrap();
    std::fs::create_dir(root.path().join("source")).unwrap();
    std::fs::copy(root.path().join("old-source/SKILL.md"), root.path().join("source/SKILL.md"))
        .unwrap();
    worker(root.path(), "reject");
}

fn phase_is_foreign() -> bool {
    std::env::var("ACP_MOUNT_RECOVERY_PHASE").as_deref() == Ok("foreign")
}
fn record_path(root: &std::path::Path) -> std::path::PathBuf {
    walkdir::WalkDir::new(root.join("home"))
        .into_iter()
        .map(Result::unwrap)
        .find(|entry| {
            entry.file_type().is_file()
                && entry.path().parent().unwrap().file_name().unwrap() == "acp_session_mounts"
        })
        .expect("persisted mount record")
        .into_path()
}
#[test]
fn acp_mount_recovery_corrupt_and_foreign_records_rejected() {
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "restore");
    worker(root.path(), "foreign");
    std::fs::write(record_path(root.path()), "{broken").unwrap();
    worker(root.path(), "corrupt");
}
#[test]
fn acp_mount_recovery_persistence_failure_retains_old_revision() {
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "rollback");
    worker(root.path(), "no_mcp");
}

const CREDENTIAL_SENTINEL: &str = "RECOVERY_SECRET_NEVER_PERSIST_8";
fn mcp(root: &std::path::Path) -> agent_client_protocol::McpServer {
    use agent_client_protocol::{EnvVariable, McpServer, McpServerStdio};
    let script = root.join("read_mcp.py");
    std::fs::write(&script, r#"
import json,sys
from pathlib import Path
for line in sys.stdin:
    r=json.loads(line); m=r.get('method')
    if m == 'initialize': out={'protocolVersion':r['params']['protocolVersion'],'capabilities':{'tools':{}},'serverInfo':{'name':'recovery','version':'1'}}
    elif m == 'tools/list': out={'tools':[{'name':'read','description':'Read real source attachment','inputSchema':{'type':'object'}}]}
    elif m == 'tools/call': out={'content':[{'type':'text','text':Path(sys.argv[1]).read_text()}],'isError':False}
    else: out={}
    if 'id' in r: print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':out}),flush=True)
"#).unwrap();
    McpServer::Stdio(
        McpServerStdio::new("recovery", "python3")
            .args(vec![
                script.to_str().unwrap().into(),
                root.join("attachment.txt").to_str().unwrap().into(),
            ])
            .env(vec![EnvVariable::new("AUTHORIZATION", CREDENTIAL_SENTINEL)]),
    )
}
async fn assert_native_call(backend: &VibeWindowBackend, id: &str) {
    let (_, tools) = backend.mounted_runtime(id);
    assert_eq!(tools.tools().len(), 1);
    let result = tools.tools()[0].call(serde_json::json!({})).await.unwrap();
    assert!(result.is_success());
    assert!(result.model_text().contains("native disk attachment"));
}
#[test]
fn acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk() {
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "no_mcp");
    worker(root.path(), "restore");
    for entry in walkdir::WalkDir::new(root.path().join("home")).into_iter().map(Result::unwrap) {
        if entry.file_type().is_file() {
            let bytes = std::fs::read(entry.path()).unwrap();
            assert!(
                !bytes
                    .windows(CREDENTIAL_SENTINEL.len())
                    .any(|w| w == CREDENTIAL_SENTINEL.as_bytes()),
                "credential leaked to state file"
            );
        }
    }
}

#[test]
fn acp_mount_recovery_explicit_load_replaces_and_clears() {
    let root = fixture();
    worker(root.path(), "create");
    std::fs::create_dir(root.path().join("replacement")).unwrap();
    std::fs::write(
        root.path().join("replacement/SKILL.md"),
        "---\nname: recovery_probe\ndescription: replacement\n---\nReplacement native source\n",
    )
    .unwrap();
    worker(root.path(), "replace");
    worker(root.path(), "replaced_restore");
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "explicit_clear");
    worker(root.path(), "empty");
}
#[test]
fn acp_mount_recovery_revoked_and_edited_sources_rejected() {
    let root = fixture();
    worker(root.path(), "create");
    std::fs::write(
        root.path().join("source/SKILL.md"),
        "---\nname: recovery_probe\ndescription: edited\n---\nChanged source\n",
    )
    .unwrap();
    worker(root.path(), "reject");
    std::fs::rename(root.path().join("source"), root.path().join("revoked")).unwrap();
    worker(root.path(), "reject");
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
fn acp_mount_recovery_committed_revision_survives_exit_before_publication() {
    let root = fixture();
    worker(root.path(), "create");
    worker(root.path(), "crash_commit");
    worker(root.path(), "empty");
}
