//! 真实会话持久化、网关权限及首次响应边界。
use super::{PlanTaskEntry, PlanTasksDocument, SessionRunRequest, SessionRunner};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore};
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};
#[derive(Clone)]
struct PlanFixture {
    plan_dir: PathBuf,
    tasks: Vec<PlanTaskEntry>,
}

#[derive(Clone)]
struct GatedStreamingSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    plan_dir: PathBuf,
    subtask_release: Arc<Semaphore>,
}

impl GatedStreamingSessionRunner {
    fn new(plan_fixture: PlanFixture) -> Self {
        Self {
            plan_dir: plan_fixture.plan_dir.clone(),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            subtask_release: Arc::new(Semaphore::new(0)),
        }
    }
}

#[async_trait]
impl SessionRunner for GatedStreamingSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        if request.session_id.ends_with("-plan") {
            let fixture =
                self.plan_fixture.lock().expect("fixture lock").take().expect("planning fixture");
            write_plan_fixture(&request.root, &fixture)?;
            return Ok("plan ready".to_string());
        }

        if let Some(delta_tx) = delta_tx {
            delta_tx
                .send(format!("live:{}", request.session_id))
                .await
                .map_err(|_| "gated delta receiver closed".to_string())?;
        }

        self.subtask_release
            .acquire()
            .await
            .map_err(|_| "subtask release closed".to_string())?
            .forget();
        complete_task_mds(&self.plan_dir);
        Ok(format!("done:{}", request.session_id))
    }
}

fn complete_task_mds(plan_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(plan_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let task_file = entry.path();
        if task_file.extension().and_then(|ext| ext.to_str()) == Some("md")
            && task_file.file_name().and_then(|name| name.to_str()) != Some("plan.md")
            && task_file.file_name().and_then(|name| name.to_str()) != Some("prompt.md")
        {
            let _ = std::fs::write(
                task_file,
                "# 任务\n\n状态：已完成\n\n完成条件：子任务已实现\n\n验证命令：cargo test -p vw-agent --lib\n\n验证结果：退出码：0；通过\n",
            );
        }
    }
}

fn write_plan_fixture(_root: &str, fixture: &PlanFixture) -> Result<(), String> {
    let plan_abs = &fixture.plan_dir;
    std::fs::create_dir_all(&plan_abs)
        .map_err(|error| format!("create plan dir failed: {error}"))?;

    let doc = PlanTasksDocument {
        plan_dir: plan_abs.to_string_lossy().to_string(),
        tasks: fixture.tasks.clone(),
    };
    let tasks_json = serde_json::to_string_pretty(&doc)
        .map_err(|error| format!("serialize tasks.json failed: {error}"))?;
    std::fs::write(plan_abs.join("tasks.json"), tasks_json)
        .map_err(|error| format!("write tasks.json failed: {error}"))?;
    std::fs::write(plan_abs.join("plan.md"), "# plan\n")
        .map_err(|error| format!("write plan.md failed: {error}"))?;
    std::fs::write(plan_abs.join("prompt.md"), "# prompt\n")
        .map_err(|error| format!("write prompt.md failed: {error}"))?;

    for task in &fixture.tasks {
        let path = plan_abs.join(&task.file);
        std::fs::write(&path, format!("# {}\n\nstatus: 未开始\n", task.title))
            .map_err(|error| format!("write {} failed: {error}", path.display()))?;
    }
    Ok(())
}

fn entry(
    id: &str,
    title: &str,
    file: &str,
    batch: u32,
    depends_on_batches: Vec<u32>,
) -> PlanTaskEntry {
    PlanTaskEntry {
        id: id.to_string(),
        title: title.to_string(),
        file: file.to_string(),
        batch,
        depends_on_batches,
        status: "未开始".to_string(),
    }
}

fn plan_fixture(home: &Path, task_id: &str, tasks: Vec<PlanTaskEntry>) -> PlanFixture {
    PlanFixture { plan_dir: super::plan_dir_abs(home, task_id), tasks }
}

async fn run_task_plan<R, F>(
    task_id: &str,
    request: &TaskPlanRunRequest,
    runner: &R,
    on_event: F,
) -> Result<(), String>
where
    R: SessionRunner + ?Sized,
    F: FnMut(TaskPlanRunStreamEventDto) -> bool,
{
    let home = PathBuf::from(&request.root);
    super::runner::run_task_plan_in_home(task_id, request, runner, &home, on_event).await
}

fn assert_subtask_delta_precedes_finished(
    events: &[TaskPlanRunStreamEventDto],
    expected_session_id: &str,
) {
    let delta_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                TaskPlanRunStreamEventDto::SessionDelta { session_id, .. }
                    if session_id == expected_session_id
            )
        })
        .expect("subtask delta");
    let finished_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
                    if session_id == expected_session_id
            )
        })
        .expect("subtask finished");
    assert!(
        delta_index < finished_index,
        "{expected_session_id} delta must precede its finished event"
    );
}

#[tokio::test]
async fn run_plan_streams_each_subtask_before_completion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-live-subtasks";
    let runner = GatedStreamingSessionRunner::new(plan_fixture(
        temp.path(),
        task_id,
        vec![entry("1", "并发 A", "1-a.md", 1, vec![]), entry("2", "并发 B", "2-b.md", 1, vec![])],
    ));
    let request = TaskPlanRunRequest {
        query: "实时查看并发子任务".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-chat".to_string()),
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let started_info_available = Arc::new(Mutex::new(Vec::new()));
    let event_notify = Arc::new(Notify::new());

    let run_runner = runner.clone();
    let run_events = Arc::clone(&events);
    let run_started_info_available = Arc::clone(&started_info_available);
    let run_event_notify = Arc::clone(&event_notify);
    let run_request = request.clone();
    let run_task_id = task_id.to_string();
    let handle = tokio::spawn(async move {
        run_task_plan(&run_task_id, &run_request, &run_runner, |event| {
            if let TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. } = &event {
                run_started_info_available
                    .lock()
                    .expect("started info")
                    .push(crate::session::ui_store::load_agent_session_any(session_id).is_some());
            }
            run_events.lock().expect("events").push(event);
            run_event_notify.notify_one();
            true
        })
        .await
    });

    let streamed_before_release = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let delta_count = events
                .lock()
                .expect("events")
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        TaskPlanRunStreamEventDto::SessionDelta { session_id, .. }
                            if session_id == "demo-live-subtasks-1"
                                || session_id == "demo-live-subtasks-2"
                    )
                })
                .count();
            if delta_count == 2 {
                break;
            }
            event_notify.notified().await;
        }
    })
    .await
    .is_ok();

    runner.subtask_release.add_permits(2);
    handle.await.expect("run task join").expect("run_task_plan should succeed");

    assert!(
        streamed_before_release,
        "each subtask delta must be forwarded while both sessions are still running"
    );
    assert_eq!(
        started_info_available.lock().expect("started info").as_slice(),
        &[true, true],
        "SubtaskStarted must only be emitted after session Info is persisted"
    );

    let events = events.lock().expect("events");
    for session_id in ["demo-live-subtasks-1", "demo-live-subtasks-2"] {
        assert_subtask_delta_precedes_finished(&events, session_id);
    }
}


#[test]
fn gateway_session_options_always_enable_full_access() {
    let request = SessionRunRequest {
        session_id: "s1".to_string(),
        query: "q".to_string(),
        root: "/tmp/project".to_string(),
        model: Some("m".to_string()),
        agent: Some("main".to_string()),
        acp_agent: None,
    };
    let options = super::gateway_session_options(&request, false);
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("cwd").and_then(serde_json::Value::as_str), Some("/tmp/project"));
    assert_eq!(options.get("agent").and_then(serde_json::Value::as_str), Some("main"));
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(options.get("goal_mode").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("workflow_mode").and_then(serde_json::Value::as_bool), Some(true));
    assert!(!options.contains_key("acp_permission_mode"));
}


#[test]
fn acp_background_approval_plan_run_defaults_to_approval() {
    let request = SessionRunRequest {
        session_id: "s1".to_string(),
        query: "q".to_string(),
        root: "/tmp/project".to_string(),
        model: None,
        agent: None,
        acp_agent: Some("codex".to_string()),
    };
    let options = super::gateway_session_options(&request, false);
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(options.get("acp_agent").and_then(serde_json::Value::as_str), Some("codex"));
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-reads")
    );
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(true));
}


#[test]
fn gateway_session_options_respects_confirmed_acp_full_access() {
    let request = SessionRunRequest {
        session_id: "s1".into(),
        query: "q".into(),
        root: "/tmp/project".into(),
        model: None,
        agent: None,
        acp_agent: Some("codex".into()),
    };
    let options = super::gateway_session_options(&request, true);
    assert_eq!(options["full_access"], true);
    assert_eq!(options["acp_permission_mode"], "approve-all");
    assert_eq!(options["acp_non_interactive_permissions"], "deny");
}


#[tokio::test]
async fn acp_background_approval_plan_inherits_current_workspace_grant() {
    use crate::session::agent_binding::{AgentBinding, resolve_full_access_scoped, save_scoped};
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().to_str().unwrap();
    let parent = format!("parent-{}", uuid::Uuid::new_v4());
    super::session_bind::ensure_plan_run_session_info(&parent, None, root).await.unwrap();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let definition = config_agent::AgentDefinitionConfig {
        runtime: config_agent_profile::AgentRuntime::Acp { backend: "codex".into() },
        ..Default::default()
    };
    let binding = AgentBinding::from_definition("acp-codex".into(), &definition);
    save_scoped(&project.id, &parent, &binding).await.unwrap();
    let runner = super::GatewaySessionRunner::new(tenant, Some(parent.clone()));
    let mut request = SessionRunRequest {
        session_id: "child".into(),
        query: "q".into(),
        root: root.into(),
        model: None,
        agent: Some("acp-codex".into()),
        acp_agent: Some("codex".into()),
    };
    assert!(!runner.acp_full_access(&request).await.unwrap());
    resolve_full_access_scoped(&project.id, &parent, Some(true), false).await.unwrap();
    assert!(runner.acp_full_access(&request).await.unwrap());

    request.acp_agent = Some("claude".into());
    assert!(!runner.acp_full_access(&request).await.unwrap());
    request.acp_agent = Some("codex".into());
    let other = tempfile::tempdir().unwrap();
    request.root = other.path().to_string_lossy().into_owned();
    assert!(!runner.acp_full_access(&request).await.unwrap());

    request.root = root.into();
    resolve_full_access_scoped(&project.id, &parent, Some(false), false).await.unwrap();
    assert!(!runner.acp_full_access(&request).await.unwrap());
}


#[test]
fn gateway_plan_run_preallocates_assistant_message_id() {
    let first =
        super::runner::preallocate_assistant_message_id().expect("first assistant message id");
    let second =
        super::runner::preallocate_assistant_message_id().expect("second assistant message id");

    assert!(!first.trim().is_empty());
    assert!(!second.trim().is_empty());
    assert_ne!(first, second);
}


#[tokio::test]
async fn plan_run_first_response_publishes_only_successful_stage_history() {
    use crate::session::ui_store;

    struct StageRunner {
        fixture: PlanFixture,
        project_id: String,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl SessionRunner for StageRunner {
        async fn run_session(
            &self,
            request: SessionRunRequest,
            _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
        ) -> Result<String, String> {
            let id = &request.session_id;
            let scope = Some(self.project_id.as_str());
            let info = ui_store::load_agent_session_scoped(id, scope).expect("reserved stage");
            assert_eq!(info.parent_id.as_deref(), Some("parent-chat"));
            assert!(info.is_plan_run());
            assert!(info.title.is_empty(), "{id} has a title before responding");
            assert!(info.is_first_response_pending());
            assert!(!ui_store::load_agent_sessions_scoped(scope).iter().any(|info| info.id == *id));
            let chat = ui_store::load_session_scoped(id, scope).expect("pending history index");
            assert!(chat.title.is_empty());
            assert!(chat.messages.is_empty());
            assert!(!ui_store::load_sessions_scoped(scope).iter().any(|meta| meta.id == *id));

            let mut calls = self.calls.lock().unwrap();
            calls.push(id.clone());
            match calls.len() {
                1 => Err("planning failure".into()),
                2 => {
                    write_plan_fixture(&request.root, &self.fixture)?;
                    Ok("plan ready".into())
                }
                3 => Err("execution failure".into()),
                4 => Ok("execution ready".into()),
                5 => Err("evaluation failure".into()),
                6 => {
                    std::fs::write(
                        self.fixture.plan_dir.join(&self.fixture.tasks[0].file),
                        "# 任务\n\n状态：已完成\n\n完成条件：子任务已实现\n\n验证命令：cargo test -p vw-agent --lib\n\n验证结果：退出码：0；通过\n",
                    )
                    .unwrap();
                    Ok("evaluation ready".into())
                }
                _ => panic!("unexpected stage: {id}"),
            }
        }
    }

    let temp = tempfile::tempdir().unwrap();
    let task_id = "first-response";
    let tenant = ui_store::current_tenant_scope();
    let project_id = crate::project::from_directory(&tenant, temp.path()).await.unwrap().0.id;
    let runner = StageRunner {
        fixture: plan_fixture(
            temp.path(),
            task_id,
            vec![entry("1", "执行标签", "1-stage.md", 1, vec![])],
        ),
        project_id: project_id.clone(),
        calls: Mutex::new(Vec::new()),
    };
    let request = TaskPlanRunRequest {
        query: "run plan".into(),
        root: temp.path().to_string_lossy().into_owned(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("parent-chat".into()),
    };
    let first =
        super::runner::run_task_plan_in_home(task_id, &request, &runner, temp.path(), |_| true)
            .await
            .unwrap_err();
    assert!(first.contains("planning failure"));

    let mut events = Vec::new();
    super::runner::run_task_plan_in_home(task_id, &request, &runner, temp.path(), |event| {
        events.push(event);
        true
    })
    .await
    .unwrap();

    let planning = format!("{task_id}-plan");
    let execution = format!("{task_id}-1");
    let failed_evaluation = super::goal::goal_turn_session_id(task_id, 2);
    let successful_evaluation = super::goal::goal_turn_session_id(task_id, 3);
    assert_eq!(
        *runner.calls.lock().unwrap(),
        [
            planning.clone(),
            planning.clone(),
            execution.clone(),
            execution.clone(),
            failed_evaluation.clone(),
            successful_evaluation.clone()
        ],
    );
    let scope = Some(project_id.as_str());
    for (id, title, output) in [
        (planning, "规划 · first-response", "plan ready"),
        (execution, "执行标签", "execution ready"),
        (successful_evaluation, "目标续轮 · 第 3 轮", "evaluation ready"),
    ] {
        let info = ui_store::load_agent_session_scoped(&id, scope).expect("committed Info");
        assert!(!info.is_first_response_pending());
        assert_eq!(info.title, title);
        let chat = ui_store::load_session_scoped(&id, scope).expect("committed history");
        assert_eq!(chat.messages[1].content, output);
    }
    let failed = ui_store::load_agent_session_scoped(&failed_evaluation, scope).unwrap();
    assert!(failed.is_first_response_pending());
    assert!(failed.title.is_empty());
    let chat = ui_store::load_session_scoped(&failed_evaluation, scope).unwrap();
    assert!(chat.title.is_empty());
    assert!(chat.messages.is_empty());
    assert!(!ui_store::load_sessions_scoped(scope).iter().any(|meta| meta.id == failed_evaluation));
    assert!(matches!(events.last(), Some(TaskPlanRunStreamEventDto::Finished { output, .. })
        if output.contains("evaluation failure") && output.contains("evaluation ready")));
}
