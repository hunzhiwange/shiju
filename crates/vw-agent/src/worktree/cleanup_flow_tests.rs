use super::registry;
use serde_json::json;
use std::path::{Path, PathBuf};
use {
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
use config_worktree::WorktreeSettings;

// A process owns all lazy stores and environment variables. Never mutate a live
// test runner's environment or share tenant databases with a parallel fixture.
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_WORKTREE_CLEANUP_FLOW_CHILD";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return true;
    }
    let root = tempfile::tempdir().unwrap();
    let name = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &name, "--nocapture"])
        .env(CHILD, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", root.path().join("no-global-gitconfig"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    print!("{stdout}");
    eprint!("{}", String::from_utf8_lossy(&result.stderr));
    assert!(stdout.contains("running 1 test"), "child must execute one real test: {test}");
    assert!(result.status.success(), "isolated cleanup flow failed: {test}");
    false
}

fn git(path: &Path, args: &[&str]) {
    let output = std::process::Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
struct Fixture {
    temp: tempfile::TempDir,
    primary: PathBuf,
    scope: ProjectScope,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("primary");
        std::fs::create_dir(&primary).unwrap();
        git(&primary, &["init", "-b", "main"]);
        git(
            &primary,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "--allow-empty",
                "-m",
                "initial",
            ],
        );
        let tenant = TenantScope::new("cleanup-flow", uuid::Uuid::new_v4().to_string()).unwrap();
        let (project, _) = crate::project::from_directory(&tenant, &primary).await.unwrap();
        let scope = ProjectScope::new(tenant.clone(), project.id.into()).unwrap();
        let settings = WorktreeSettings {
            keep_count: 1,
            root: temp.path().join("restored").to_string_lossy().into_owned(),
            ..Default::default()
        };
        crate::config::update_global_for_tenant(&tenant, json!({"worktree_settings": settings}))
            .await
            .unwrap();
        Self { temp, primary, scope }
    }
    async fn checkout(&self, name: &str) -> registry::Record {
        let path = self.temp.path().join(name);
        git(&self.primary, &["worktree", "add", "-b", name, path.to_str().unwrap()]);
        registry::register(&self.scope, &self.primary, &path).await.unwrap()
    }

}

fn request(root: &Path) -> crate::session::processor::Request {
    crate::session::processor::Request {
        stream: 1,
        session: uuid::Uuid::new_v4().to_string(),
        query: "hello".into(),
        root: Some(root.to_string_lossy().into_owned()),
        model: None,
        reliability: Default::default(),
        options: json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: vec![],
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
        session_skills: Default::default(),
    }
}

#[tokio::test]
async fn worktree_cleanup_flow_runtime_and_detached_shell_hold_leases() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_runtime_and_detached_shell_hold_leases") {
        return;
    }
    use crate::session::{
        processor::StreamEvent,
        runtime::{PromptEvent, start_prompt_with_runner},
    };
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcesses};
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let (started, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let turn = start_prompt_with_runner(request(&old.directory), move |_, mut emit| {
        started.send(()).unwrap();
        wait.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        emit(StreamEvent::Done(Default::default()));
    });
    ready.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
    assert!(registry::ensure_not_in_use(&old).is_err());
    release.send(()).unwrap();
    loop {
        if matches!(
            turn.events.recv_timeout(std::time::Duration::from_secs(10)).unwrap(),
            PromptEvent::Terminated(_)
        ) {
            break;
        }
    }
    for _ in 0..100 {
        if registry::ensure_not_in_use(&old).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(registry::ensure_not_in_use(&old).is_ok());
    let host = ShellProcesses::new::<crate::tools::context::AgentContextHost>().unwrap();
    let scope = ShellProcessScope::new("fixture".into(), &f.primary, "detached".into()).unwrap();
    let cwd = old.directory.clone();
    let process = host
        .start(scope.clone(), move || {
            let mut command = crate::shell::tokio_system_command("git");
            command.args(["-c", "alias.hold=!sleep 30", "hold"]).current_dir(cwd);
            Ok(command)
        })
        .await
        .unwrap();
    assert!(registry::ensure_not_in_use(&old).is_err());
    host.cancel(&scope, &process.process_id).await.unwrap();
    host.shutdown().unwrap();
    for _ in 0..100 {
        if registry::ensure_not_in_use(&old).is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("execution lease was not released after runtime and child exit");
}

#[tokio::test]
async fn worktree_cleanup_flow_session_creation_cannot_cross_removal() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_session_creation_cannot_cross_removal") {
        return;
    }
    use crate::session::runtime::{CreateSessionInput, SessionScope, create_session};
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let scope = SessionScope::new(f.scope.tenant_scope().clone(), &old.directory).unwrap();
    let lock = registry::lifecycle_lock(&f.primary).unwrap();
    assert!(
        create_session(CreateSessionInput {
            scope,
            parent_id: None,
            title: None,
            permission: None
        })
        .await
        .is_err()
    );
    drop(lock);
    assert!(old.directory.exists());
}

struct LeaseProbeProvider(registry::Record);
#[async_trait::async_trait]
impl model_types::Provider for LeaseProbeProvider {
    async fn chat_with_system(
        &self,
        _system: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        assert!(
            registry::ensure_not_in_use(&self.0).is_err(),
            "legacy execution must hold its workspace lease"
        );
        Ok("done".into())
    }
}
async fn legacy_turn(record: &registry::Record) -> anyhow::Result<String> {
    let provider = LeaseProbeProvider(record.clone());
    let policy = std::sync::Arc::new(crate::security::SecurityPolicy {
        workspace_dir: record.directory.clone(),
        ..Default::default()
    });
    let mut history = vec![model_types::ChatMessage::user("hello")];
    crate::agent::loop_::core::run_tool_call_loop(
        &provider,
        &mut history,
        &[],
        &crate::observability::NoopObserver,
        "fixture",
        "fixture",
        0.0,
        true,
        None,
        "cli",
        &crate::config::MultimodalConfig::default(),
        1,
        None,
        None,
        None,
        Some(policy),
        &[],
    )
    .await
}
#[tokio::test]
async fn worktree_cleanup_flow_legacy_execution_cannot_cross_removal() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_legacy_execution_cannot_cross_removal") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let removal = registry::lifecycle_lock(&f.primary).unwrap();
    let error = legacy_turn(&old).await.unwrap_err();
    assert!(error.to_string().contains("Worktree lifecycle is busy"), "{error}");
    drop(removal);
    assert_eq!(legacy_turn(&old).await.unwrap(), "done");
    assert!(registry::ensure_not_in_use(&old).is_ok());
}

#[tokio::test]
async fn worktree_cleanup_flow_binding_retains_lease_until_cleared() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_binding_retains_lease_until_cleared") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let context = crate::tools::ToolUseContext::new(
        "binding",
        Some(f.primary.to_string_lossy().into_owned()),
    );
    context
        .bind_worktree(old.directory.to_string_lossy().into_owned(), "old".into(), "old".into())
        .unwrap();
    assert!(
        registry::ensure_not_in_use(&old).is_err(),
        "a bound worktree must remain protected between tool calls"
    );
    let metadata = context.workflow_state();
    let other = f.checkout("other").await;
    assert!(
        context
            .bind_worktree(
                other.directory.to_string_lossy().into_owned(),
                "other".into(),
                "other".into()
            )
            .is_err()
    );
    assert!(registry::ensure_not_in_use(&old).is_err());
    assert!(registry::ensure_not_in_use(&other).is_ok());
    context.clear_worktree_binding();
    assert!(
        registry::ensure_not_in_use(&old).is_ok(),
        "metadata snapshots must not retain live leases"
    );
    assert_eq!(metadata.worktree.directory.as_deref(), old.directory.to_str());
    assert_eq!(serde_json::to_value(metadata).unwrap().as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn worktree_cleanup_flow_binding_rejects_removal_without_changing_context() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_binding_rejects_removal_without_changing_context") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let context = crate::tools::ToolUseContext::new(
        "binding",
        Some(f.primary.to_string_lossy().into_owned()),
    );
    let removal = registry::lifecycle_lock(&f.primary).unwrap();
    assert!(
        context
            .bind_worktree(old.directory.to_string_lossy().into_owned(), "old".into(), "old".into())
            .is_err(),
        "binding must not cross an in-flight removal"
    );
    assert!(context.worktree_binding_state().directory.is_none());
    drop(removal);
    assert!(registry::ensure_not_in_use(&old).is_ok());
}
