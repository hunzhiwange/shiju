//! `run_task_plan` 注入式 SessionRunner 行为测试。

use super::{
    PlanTaskEntry, PlanTasksDocument, SessionRunRequest, SessionRunner, run_task_plan,
    run_task_plan_with_cancellation,
};
use crate::session::child_agent::coordinator::global_child_agent_coordinator;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore};
use tokio_util::sync::CancellationToken;
use vw_api_types::task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

#[derive(Clone)]
struct ScriptedSessionRunner {
    /// 按调用顺序返回的结果；若带 fixture，首次规划调用会先把文件写到计划目录。
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
}

#[derive(Clone)]
struct PlanFixture {
    plan_dir_rel: String,
    tasks: Vec<PlanTaskEntry>,
}

#[derive(Clone)]
struct GatedStreamingSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    subtask_release: Arc<Semaphore>,
}

#[derive(Clone)]
struct GoalSessionRunner {
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    task_files: Arc<Mutex<Vec<PathBuf>>>,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
    delete_on_continuation: bool,
    initial_blocker: Option<String>,
}

#[derive(Clone)]
struct DescendantGatedSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    descendant_started: Arc<Semaphore>,
    descendant_release: Arc<Semaphore>,
}

#[derive(Clone)]
struct GoalBoundarySessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    task_files: Arc<Mutex<Vec<PathBuf>>>,
    call_count: Arc<Mutex<usize>>,
    descendant_started: Arc<Semaphore>,
    descendant_release: Arc<Semaphore>,
}

#[derive(Clone)]
struct CancellationGatedSessionRunner {
    started: Arc<Semaphore>,
}

impl Default for CancellationGatedSessionRunner {
    fn default() -> Self {
        Self { started: Arc::new(Semaphore::new(0)) }
    }
}

impl ScriptedSessionRunner {
    fn new(responses: Vec<Result<String, String>>, plan_fixture: PlanFixture) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn without_plan_files(responses: Vec<Result<String, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(None)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl GatedStreamingSessionRunner {
    fn new(plan_fixture: PlanFixture) -> Self {
        Self {
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            subtask_release: Arc::new(Semaphore::new(0)),
        }
    }
}

impl GoalSessionRunner {
    fn new(
        responses: Vec<Result<String, String>>,
        plan_fixture: PlanFixture,
        delete_on_continuation: bool,
        initial_blocker: Option<&str>,
    ) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            task_files: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            delete_on_continuation,
            initial_blocker: initial_blocker.map(str::to_string),
        }
    }
}

impl DescendantGatedSessionRunner {
    fn new(plan_fixture: PlanFixture) -> Self {
        Self {
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            descendant_started: Arc::new(Semaphore::new(0)),
            descendant_release: Arc::new(Semaphore::new(0)),
        }
    }
}

impl GoalBoundarySessionRunner {
    fn new(plan_fixture: PlanFixture) -> Self {
        Self {
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            task_files: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(Mutex::new(0)),
            descendant_started: Arc::new(Semaphore::new(0)),
            descendant_release: Arc::new(Semaphore::new(0)),
        }
    }
}

#[async_trait]
impl SessionRunner for CancellationGatedSessionRunner {
    async fn run_session(
        &self,
        _request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        self.started.add_permits(1);
        std::future::pending().await
    }
}

#[async_trait]
impl SessionRunner for ScriptedSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        self.calls.lock().expect("calls lock").push(request.clone());

        if let Some(delta_tx) = delta_tx {
            delta_tx
                .send(format!("live:{}", request.session_id))
                .await
                .map_err(|_| "scripted delta receiver closed".to_string())?;
        }

        // 首次规划调用：在计划目录落盘 tasks.json / plan.md / 任务文件。
        if let Some(fixture) = self.plan_fixture.lock().expect("fixture lock").take() {
            write_plan_fixture(&request.root, &fixture)?;
        }

        self.responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err("scripted session runner: no more responses".to_string()))
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
            let fixture = self
                .plan_fixture
                .lock()
                .expect("fixture lock")
                .take()
                .expect("planning fixture");
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
        Ok(format!("done:{}", request.session_id))
    }
}

#[async_trait]
impl SessionRunner for GoalSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let call_index = {
            let mut calls = self.calls.lock().expect("calls lock");
            calls.push(request.clone());
            calls.len()
        };

        if let Some(fixture) = self.plan_fixture.lock().expect("fixture lock").take() {
            write_plan_fixture(&request.root, &fixture)?;
            let plan_abs = PathBuf::from(&request.root).join(&fixture.plan_dir_rel);
            *self.task_files.lock().expect("task files lock") =
                fixture.tasks.iter().map(|task| plan_abs.join(&task.file)).collect();
        } else if call_index == 2 {
            if let Some(reason) = &self.initial_blocker {
                let task_file = self
                    .task_files
                    .lock()
                    .expect("task files lock")
                    .first()
                    .cloned()
                    .expect("task file");
                std::fs::write(
                    task_file,
                    format!("# 任务\n\n状态：已阻塞\n\n阻塞原因：{reason}\n"),
                )
                .map_err(|error| format!("write blocker failed: {error}"))?;
            }
        } else if self.delete_on_continuation && request.query.contains("目标续轮") {
            for task_file in self.task_files.lock().expect("task files lock").iter() {
                if task_file.exists() {
                    std::fs::remove_file(task_file)
                        .map_err(|error| format!("delete task file failed: {error}"))?;
                }
            }
        }

        self.responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err("goal session runner: no more responses".to_string()))
    }
}

#[async_trait]
impl SessionRunner for DescendantGatedSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        if request.session_id.ends_with("-plan") {
            let fixture = self
                .plan_fixture
                .lock()
                .expect("fixture lock")
                .take()
                .expect("planning fixture");
            write_plan_fixture(&request.root, &fixture)?;
            return Ok("plan ready".to_string());
        }

        let coordinator = global_child_agent_coordinator();
        let child_session_id = format!("{}-child", request.session_id);
        let descendant_session_id = format!("{}-nested", request.session_id);
        coordinator
            .register(
                &request.session_id,
                &child_session_id,
                "planner",
                "coordinate nested verification",
            )
            .map_err(|error| error.to_string())?;
        coordinator
            .mark_succeeded(
                &request.session_id,
                &child_session_id,
                "delegated verification",
            )
            .map_err(|error| error.to_string())?;
        coordinator
            .register(
                &child_session_id,
                &descendant_session_id,
                "reviewer",
                "verify parent output",
            )
            .map_err(|error| error.to_string())?;
        coordinator
            .mark_running(&child_session_id, &descendant_session_id)
            .map_err(|error| error.to_string())?;
        self.descendant_started.add_permits(1);

        let descendant_release = Arc::clone(&self.descendant_release);
        tokio::spawn(async move {
            descendant_release
                .acquire()
                .await
                .expect("descendant release should remain open")
                .forget();
            if request.session_id.ends_with("-2") {
                coordinator
                    .mark_failed(
                        &child_session_id,
                        &descendant_session_id,
                        "nested verification failed",
                    )
                    .expect("descendant should fail");
            } else if request.session_id.ends_with("-3") {
                coordinator
                    .cancel(&child_session_id, &descendant_session_id)
                    .expect("descendant should cancel");
            } else {
                coordinator
                    .mark_succeeded(
                        &child_session_id,
                        &descendant_session_id,
                        "nested verification passed",
                    )
                    .expect("descendant should complete");
            }
        });

        Ok("parent session returned".to_string())
    }
}

#[async_trait]
impl SessionRunner for GoalBoundarySessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let call_index = {
            let mut call_count = self.call_count.lock().expect("call count lock");
            *call_count += 1;
            *call_count
        };

        if let Some(fixture) = self.plan_fixture.lock().expect("fixture lock").take() {
            write_plan_fixture(&request.root, &fixture)?;
            let plan_abs = PathBuf::from(&request.root).join(&fixture.plan_dir_rel);
            *self.task_files.lock().expect("task files lock") =
                fixture.tasks.iter().map(|task| plan_abs.join(&task.file)).collect();
        } else if call_index == 4 {
            for task_file in self.task_files.lock().expect("task files lock").iter() {
                std::fs::remove_file(task_file)
                    .map_err(|error| format!("delete task file failed: {error}"))?;
            }
        }

        let coordinator = global_child_agent_coordinator();
        let descendant_session_id = format!("{}-nested", request.session_id);
        coordinator
            .register(
                &request.session_id,
                &descendant_session_id,
                "reviewer",
                "verify goal boundary",
            )
            .map_err(|error| error.to_string())?;
        coordinator
            .mark_running(&request.session_id, &descendant_session_id)
            .map_err(|error| error.to_string())?;
        self.descendant_started.add_permits(1);

        let descendant_release = Arc::clone(&self.descendant_release);
        tokio::spawn(async move {
            descendant_release
                .acquire()
                .await
                .expect("descendant release should remain open")
                .forget();
            coordinator
                .mark_succeeded(
                    &request.session_id,
                    &descendant_session_id,
                    "goal boundary verified",
                )
                .expect("goal descendant should complete");
        });

        match call_index {
            1 => Ok("plan ready".to_string()),
            2 => Ok("initial execution complete".to_string()),
            3 => Ok(r#"{"achieved":false,"reason":"continue"}"#.to_string()),
            4 => Ok("continuation complete".to_string()),
            5 => Ok(r#"{"achieved":true,"reason":"verified"}"#.to_string()),
            _ => Err(format!("unexpected goal boundary call {call_index}")),
        }
    }
}

fn write_plan_fixture(root: &str, fixture: &PlanFixture) -> Result<(), String> {
    let plan_abs = PathBuf::from(root).join(&fixture.plan_dir_rel);
    std::fs::create_dir_all(&plan_abs)
        .map_err(|error| format!("create plan dir failed: {error}"))?;

    let doc = PlanTasksDocument {
        plan_dir: fixture.plan_dir_rel.clone(),
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

fn event_kind(event: &TaskPlanRunStreamEventDto) -> &'static str {
    match event {
        TaskPlanRunStreamEventDto::PlanningStarted { .. } => "planning_started",
        TaskPlanRunStreamEventDto::SessionDelta { .. } => "session_delta",
        TaskPlanRunStreamEventDto::SessionWaiting { .. } => "session_waiting",
        TaskPlanRunStreamEventDto::PlanCreated { .. } => "plan_created",
        TaskPlanRunStreamEventDto::BatchStarted { .. } => "batch_started",
        TaskPlanRunStreamEventDto::BatchFinished { .. } => "batch_finished",
        TaskPlanRunStreamEventDto::GoalTurnStarted { .. } => "goal_turn_started",
        TaskPlanRunStreamEventDto::GoalEvaluated { .. } => "goal_evaluated",
        TaskPlanRunStreamEventDto::SubtaskStarted { .. } => "subtask_started",
        TaskPlanRunStreamEventDto::SubtaskFinished { .. } => "subtask_finished",
        TaskPlanRunStreamEventDto::Blocked { .. } => "blocked",
        TaskPlanRunStreamEventDto::Finished { .. } => "finished",
        TaskPlanRunStreamEventDto::Error { .. } => "error",
        TaskPlanRunStreamEventDto::Cancelled { .. } => "cancelled",
    }
}

fn subtask_id_of(event: &TaskPlanRunStreamEventDto) -> Option<&str> {
    match event {
        TaskPlanRunStreamEventDto::SubtaskStarted { subtask_id, .. }
        | TaskPlanRunStreamEventDto::SubtaskFinished { subtask_id, .. } => Some(subtask_id.as_str()),
        TaskPlanRunStreamEventDto::Blocked {
            subtask_id: Some(subtask_id),
            ..
        } => Some(subtask_id.as_str()),
        _ => None,
    }
}

fn batch_of(event: &TaskPlanRunStreamEventDto) -> Option<u32> {
    match event {
        TaskPlanRunStreamEventDto::BatchStarted { batch, .. }
        | TaskPlanRunStreamEventDto::BatchFinished { batch, .. }
        | TaskPlanRunStreamEventDto::SubtaskStarted { batch, .. }
        | TaskPlanRunStreamEventDto::SubtaskFinished { batch, .. } => Some(*batch),
        TaskPlanRunStreamEventDto::Blocked { batch, .. } => *batch,
        _ => None,
    }
}

fn plan_dir_exists(root: &Path, rel: &str) -> bool {
    root.join(rel).join("tasks.json").is_file()
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

async fn wait_for_session_waiting(
    events: &Arc<Mutex<Vec<TaskPlanRunStreamEventDto>>>,
    expected_session_id: &str,
) {
    tokio::time::timeout(std::time::Duration::from_millis(200), async {
        loop {
            if events.lock().expect("events lock").iter().any(|event| {
                matches!(
                    event,
                    TaskPlanRunStreamEventDto::SessionWaiting { session_id, .. }
                        if session_id == expected_session_id
                )
            }) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("SessionWaiting should be emitted for {expected_session_id}"));
}

#[tokio::test]
async fn task_plan_run_root_cancellation_publishes_cancelled_once() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runner = CancellationGatedSessionRunner::default();
    let started = Arc::clone(&runner.started);
    let cancellation = CancellationToken::new();
    let cancellation_for_run = cancellation.clone();
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected_events = Arc::clone(&events);
    let request = TaskPlanRunRequest {
        query: "run until stopped".to_string(),
        root: temp.path().to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
    };

    let run = tokio::spawn(async move {
        run_task_plan_with_cancellation(
            "cancelled-plan-run",
            &request,
            &runner,
            cancellation_for_run,
            |event| {
                collected_events.lock().expect("events lock").push(event);
                true
            },
        )
        .await
    });

    tokio::time::timeout(std::time::Duration::from_millis(200), started.acquire())
        .await
        .expect("planning session should start")
        .expect("started semaphore should remain open")
        .forget();
    cancellation.cancel();
    tokio::time::timeout(std::time::Duration::from_millis(200), run)
        .await
        .expect("root cancellation should stop planning")
        .expect("plan-run task should join")
        .expect("cancellation is a terminal outcome");

    let events = events.lock().expect("events lock");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, TaskPlanRunStreamEventDto::Cancelled { .. }))
            .count(),
        1
    );
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Cancelled {
            task_id,
            session_id: None,
        }) if task_id == "cancelled-plan-run"
    ));
}

#[tokio::test]
async fn run_plan_delays_subtask_finished_until_nested_agents_are_terminal() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-nested-barrier";
    let plan_dir_rel = format!(".plans/task-{task_id}");
    let runner = DescendantGatedSessionRunner::new(PlanFixture {
        plan_dir_rel,
        tasks: vec![
            entry("1", "父任务一", "1-parent.md", 1, vec![]),
            entry("2", "父任务二", "2-parent.md", 1, vec![]),
            entry("3", "父任务三", "3-parent.md", 1, vec![]),
        ],
    });
    let descendant_started = Arc::clone(&runner.descendant_started);
    let descendant_release = Arc::clone(&runner.descendant_release);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected_events = Arc::clone(&events);

    let run = tokio::spawn(async move {
        let request = TaskPlanRunRequest {
            query: "run nested agents".to_string(),
            root,
            model: None,
            agent: None,
            acp_agent: None,
            parent_session_id: Some("main-session".to_string()),
            mode: vw_api_types::task::TaskPlanRunMode::Workflow,
        };
        run_task_plan(task_id, &request, &runner, |event| {
            collected_events.lock().expect("events lock").push(event);
            true
        })
        .await
    });

    tokio::time::timeout(std::time::Duration::from_millis(200), async {
        descendant_started
            .acquire_many(3)
            .await
            .expect("descendant start semaphore should remain open")
            .forget();
    })
    .await
    .expect("same-batch nested descendants should start");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    assert!(
        !run.is_finished(),
        "plan-run must wait while a nested descendant remains active"
    );
    assert!(
        !events
            .lock()
            .expect("events lock")
            .iter()
            .any(|event| matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. })),
        "SubtaskFinished must not be emitted before nested descendants are terminal"
    );

    descendant_release.add_permits(3);
    tokio::time::timeout(std::time::Duration::from_millis(200), run)
        .await
        .expect("terminal descendant should release plan-run")
        .expect("plan-run task should join")
        .expect("plan-run should succeed");
    assert!(
        events
            .lock()
            .expect("events lock")
            .iter()
            .any(|event| matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. })),
        "SubtaskFinished should follow descendant completion"
    );

    let events = events.lock().expect("events lock");
    let first_finished = events
        .iter()
        .position(|event| matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. }))
        .expect("first subtask finished");
    let waiting = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            TaskPlanRunStreamEventDto::SessionWaiting {
                active_descendant_session_ids,
                ..
            } => Some((index, active_descendant_session_ids)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(waiting.len(), 3);
    assert!(waiting.iter().all(|(index, _)| *index < first_finished));
    assert!(waiting.iter().all(|(_, active)| active.len() == 1));
    assert!(
        waiting
            .iter()
            .all(|(_, active)| active[0].ends_with("-nested"))
    );

    let finished = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::SubtaskFinished {
                subtask_id,
                output: Some(output),
                ..
            } => Some((subtask_id.as_str(), output.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        finished.iter().map(|(subtask_id, _)| *subtask_id).collect::<Vec<_>>(),
        vec!["1", "2", "3"]
    );
    assert!(finished[0].1.contains("state=succeeded"));
    assert!(finished[1].1.contains("state=failed"));
    assert!(finished[2].1.contains("state=cancelled"));
}

#[tokio::test]
async fn goal_mode_waits_at_every_session_boundary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-boundaries";
    let runner = GoalBoundarySessionRunner::new(PlanFixture {
        plan_dir_rel: format!(".plans/task-{task_id}"),
        tasks: vec![entry("1", "目标任务", "1-goal.md", 1, vec![])],
    });
    let descendant_started = Arc::clone(&runner.descendant_started);
    let descendant_release = Arc::clone(&runner.descendant_release);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected_events = Arc::clone(&events);

    let run = tokio::spawn(async move {
        let request = TaskPlanRunRequest {
            query: "finish the goal".to_string(),
            root,
            model: None,
            agent: None,
            acp_agent: None,
            parent_session_id: Some("main-session".to_string()),
            mode: vw_api_types::task::TaskPlanRunMode::Goal,
        };
        run_task_plan(task_id, &request, &runner, |event| {
            collected_events.lock().expect("events lock").push(event);
            true
        })
        .await
    });

    for expected_session_id in [
        "demo-goal-boundaries-plan",
        "demo-goal-boundaries-1",
        "demo-goal-boundaries-goal-eval-1",
        "demo-goal-boundaries-goal-2",
        "demo-goal-boundaries-goal-eval-2",
    ] {
        tokio::time::timeout(
            std::time::Duration::from_millis(200),
            descendant_started.acquire(),
        )
        .await
        .unwrap_or_else(|_| panic!("descendant should start for {expected_session_id}"))
        .expect("descendant start semaphore should remain open")
        .forget();
        wait_for_session_waiting(&events, expected_session_id).await;
        assert!(
            !run.is_finished(),
            "goal run must remain pending at {expected_session_id}"
        );
        descendant_release.add_permits(1);
    }

    tokio::time::timeout(std::time::Duration::from_millis(200), run)
        .await
        .expect("final goal descendant should release plan-run")
        .expect("goal plan-run task should join")
        .expect("goal plan-run should succeed");

    let events = events.lock().expect("events lock");
    let waiting_sessions = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::SessionWaiting { session_id, .. } => {
                Some(session_id.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        waiting_sessions,
        vec![
            "demo-goal-boundaries-plan",
            "demo-goal-boundaries-1",
            "demo-goal-boundaries-goal-eval-1",
            "demo-goal-boundaries-goal-2",
            "demo-goal-boundaries-goal-eval-2",
        ]
    );
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, output, .. })
            if status == "completed" && output.contains("[AgentTool 后代终态]")
    ));
}

#[tokio::test]
async fn run_plan_emits_full_success_event_sequence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-success";
    let plan_dir_rel = format!(".plans/task-{}", task_id);

    let fixture = PlanFixture {
        plan_dir_rel: plan_dir_rel.clone(),
        tasks: vec![
            entry("1", "第一批 A", "1-a.md", 1, vec![]),
            entry("2", "第一批 B", "2-b.md", 1, vec![]),
            entry("3", "第二批", "3-c.md", 2, vec![1]),
        ],
    };

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Ok("subtask 1 ok".to_string()),
            Ok("subtask 2 ok".to_string()),
            Ok("subtask 3 ok".to_string()),
        ],
        fixture,
    );

    let request = TaskPlanRunRequest {
        query: "实现完整成功路径".to_string(),
        root: root.clone(),
        model: Some("demo-model".to_string()),
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-chat".to_string()),
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await
    .expect("run_task_plan should succeed");

    assert!(plan_dir_exists(temp.path(), &plan_dir_rel));

    let events = events.lock().expect("events").clone();
    let kinds: Vec<_> = events
        .iter()
        .filter(|event| !matches!(event, TaskPlanRunStreamEventDto::SessionDelta { .. }))
        .map(event_kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            "planning_started",
            "plan_created",
            "batch_started",
            "subtask_started",
            "subtask_started",
            "subtask_finished",
            "subtask_finished",
            "batch_finished",
            "batch_started",
            "subtask_started",
            "subtask_finished",
            "batch_finished",
            "finished",
        ]
    );

    assert!(matches!(
        &events[0],
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: id,
            session_id,
            ..
        } if id == task_id && session_id == "demo-success-plan"
    ));
    assert!(matches!(
        &events[1],
        TaskPlanRunStreamEventDto::SessionDelta {
            session_id,
            delta,
            ..
        } if session_id == "demo-success-plan" && delta == "live:demo-success-plan"
    ));
    assert!(matches!(
        &events[2],
        TaskPlanRunStreamEventDto::PlanCreated {
            subtask_count: 3,
            session_id,
            ..
        } if session_id == "demo-success-plan"
    ));
    let finished_batches: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::BatchFinished { batch, success, .. } => {
                Some((*batch, *success))
            }
            _ => None,
        })
        .collect();
    assert_eq!(finished_batches, vec![(1, true), (2, true)]);
    assert!(matches!(
        &events[4],
        TaskPlanRunStreamEventDto::SubtaskStarted {
            subtask_id,
            session_id,
            ..
        } if subtask_id == "1" && session_id == "demo-success-1"
    ));
    for session_id in ["demo-success-1", "demo-success-2", "demo-success-3"] {
        assert_subtask_delta_precedes_finished(&events, session_id);
    }
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished {
            status,
            session_id: Some(session_id),
            ..
        }) if status == "completed" && session_id == "demo-success-plan"
    ));

    let calls = runner.calls.lock().expect("calls").clone();
    assert_eq!(calls.len(), 4, "1 planning + 3 subtasks");
    assert_eq!(calls[0].session_id, "demo-success-plan");
}

#[tokio::test]
async fn run_plan_streams_each_subtask_before_completion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-live-subtasks";
    let plan_dir_rel = format!(".plans/task-{task_id}");
    let runner = GatedStreamingSessionRunner::new(PlanFixture {
        plan_dir_rel,
        tasks: vec![
            entry("1", "并发 A", "1-a.md", 1, vec![]),
            entry("2", "并发 B", "2-b.md", 1, vec![]),
        ],
    });
    let request = TaskPlanRunRequest {
        query: "实时查看并发子任务".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-chat".to_string()),
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
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

    let streamed_before_release = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        async {
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
        },
    )
    .await
    .is_ok();

    runner.subtask_release.add_permits(2);
    handle
        .await
        .expect("run task join")
        .expect("run_task_plan should succeed");

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

#[tokio::test]
async fn run_plan_retries_failed_subtask_without_replanning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-recovered";
    let plan_dir_rel = format!(".plans/task-{task_id}");

    let fixture = PlanFixture {
        plan_dir_rel,
        tasks: vec![
            entry("1", "重试后恢复", "1-recover.md", 1, vec![]),
            entry("2", "继续后续批次", "2-later.md", 2, vec![1]),
        ],
    };

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Err("first transient failure".to_string()),
            Err("second transient failure".to_string()),
            Ok("subtask recovered".to_string()),
            Ok("later batch completed".to_string()),
        ],
        fixture,
    );
    let request = TaskPlanRunRequest {
        query: "失败子任务应在原工作流内重试".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await
    .expect("retry should recover the workflow");

    let events = events.lock().expect("events").clone();
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, TaskPlanRunStreamEventDto::Blocked { .. }))
    );
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, .. }) if status == "completed"
    ));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                TaskPlanRunStreamEventDto::PlanningStarted { .. }
            ))
            .count(),
        1,
        "retry must not restart planning"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                TaskPlanRunStreamEventDto::SessionDelta { session_id, delta, .. }
                    if session_id == "demo-recovered-1" && delta.contains("正在重试")
            ))
            .count(),
        2
    );

    let calls = runner.calls.lock().expect("calls").clone();
    assert_eq!(calls.len(), 5, "1 planning + 3 subtask attempts + later batch");
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.session_id == "demo-recovered-plan")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.session_id == "demo-recovered-1")
            .count(),
        3
    );
    assert!(
        calls.iter().any(|call| call.session_id == "demo-recovered-2"),
        "later batch must continue after retry succeeds"
    );
}

#[tokio::test]
async fn run_plan_emits_blocked_only_after_subtask_retries_are_exhausted() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-blocked";
    let plan_dir_rel = format!(".plans/task-{}", task_id);

    let fixture = PlanFixture {
        plan_dir_rel: plan_dir_rel.clone(),
        tasks: vec![
            entry("1", "会失败", "1-fail.md", 1, vec![]),
            entry("2", "后续批次不应启动", "2-later.md", 2, vec![1]),
        ],
    };

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Err("subtask boom 1".to_string()),
            Err("subtask boom 2".to_string()),
            Err("subtask boom 3".to_string()),
            Err("subtask boom 4".to_string()),
        ],
        fixture,
    );

    let request = TaskPlanRunRequest {
        query: "失败应阻塞后续批次".to_string(),
        root: root.clone(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    let result = run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await;

    assert!(result.is_ok(), "blocked path returns Ok after emitting Blocked");

    let events = events.lock().expect("events").clone();
    let kinds: Vec<_> = events
        .iter()
        .filter(|event| !matches!(event, TaskPlanRunStreamEventDto::SessionDelta { .. }))
        .map(event_kind)
        .collect();

    assert_eq!(kinds.first().copied(), Some("planning_started"));
    assert_eq!(kinds.get(1).copied(), Some("plan_created"));
    assert_eq!(kinds.get(2).copied(), Some("batch_started"));
    assert!(kinds.contains(&"subtask_started"));
    assert!(kinds.contains(&"subtask_finished"));
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::BatchFinished {
            batch: 1,
            success: false,
            ..
        }
    )));
    assert_eq!(kinds.last().copied(), Some("blocked"));
    assert!(!kinds.contains(&"finished"));

    // 批次 2 不得启动：不应出现 batch=2 的 SubtaskStarted。
    let later_started = events.iter().any(|event| {
        matches!(
            event,
            TaskPlanRunStreamEventDto::SubtaskStarted {
                batch: 2,
                subtask_id,
                ..
            } if subtask_id == "2"
        )
    });
    assert!(
        !later_started,
        "batch 2 SubtaskStarted must not appear after failure: {kinds:?}"
    );
    assert_subtask_delta_precedes_finished(&events, "demo-blocked-1");

    let blocked = events
        .iter()
        .find(|event| matches!(event, TaskPlanRunStreamEventDto::Blocked { .. }))
        .expect("Blocked event");
    assert_eq!(batch_of(blocked), Some(1));
    assert!(subtask_id_of(blocked).is_some());
    assert!(matches!(
        blocked,
        TaskPlanRunStreamEventDto::Blocked {
            session_id,
            subtask_id: Some(subtask_id),
            ..
        } if session_id == &format!("demo-blocked-{subtask_id}")
    ));

    let calls = runner.calls.lock().expect("calls").clone();
    // 规划 + 子任务首次执行及 3 次重试；不得再为批次 2 启动子会话。
    assert_eq!(calls.len(), 5);
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.session_id == "demo-blocked-1")
            .count(),
        4
    );
    assert!(
        !calls
            .iter()
            .any(|call| call.session_id == "demo-blocked-2" || call.session_id.ends_with("-2")),
        "must not start subtask 2 session: {:?}",
        calls.iter().map(|c| &c.session_id).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn run_plan_errors_when_planning_session_skips_writing_tasks_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-no-files";
    let runner = ScriptedSessionRunner::without_plan_files(vec![Ok(
        "这是我在聊天里写的计划，但没有用工具落盘".to_string(),
    )]);
    let request = TaskPlanRunRequest {
        query: "需要真实落盘".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Workflow,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    let result = run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await;

    assert!(result.is_err());
    let err = result.expect_err("missing tasks.json");
    assert!(
        err.contains("规划会话未在固定目录写出任务文件"),
        "error should explain missing write: {err}"
    );
    assert!(
        err.contains("没有用工具落盘"),
        "error should include planning output preview: {err}"
    );

    let events = events.lock().expect("events").clone();
    assert_eq!(event_kind(&events[0]), "planning_started");
    assert_eq!(event_kind(events.last().expect("error event")), "error");
    assert!(temp.path().join(".plans").join(format!("task-{task_id}")).is_dir());
}

#[tokio::test]
async fn goal_mode_continues_until_independent_evaluation_passes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal";
    let plan_dir_rel = format!(".plans/task-{task_id}");
    let runner = GoalSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Ok("implemented but evidence missing".to_string()),
            Ok(r#"{"achieved":false,"reason":"任务文件仍存在，缺少验证证据"}"#.to_string()),
            Ok("目标续轮完成修复并验证，exit 0".to_string()),
            Ok(r#"{"achieved":true,"reason":"任务文件已删除且验证退出码为 0"}"#.to_string()),
        ],
        PlanFixture {
            plan_dir_rel,
            tasks: vec![entry("1", "实现目标", "1-goal.md", 1, vec![])],
        },
        true,
        None,
    );
    let request = TaskPlanRunRequest {
        query: "完成目标并提供验证证据".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-chat".to_string()),
        mode: vw_api_types::task::TaskPlanRunMode::Goal,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        captured.lock().expect("events").push(event);
        true
    })
    .await
    .expect("goal run");

    let events = events.lock().expect("events");
    let evaluations = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::GoalEvaluated { turn, achieved, reason, .. } => {
                Some((*turn, *achieved, reason.as_str()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        evaluations,
        vec![
            (1, false, "任务文件仍存在，缺少验证证据"),
            (2, true, "任务文件已删除且验证退出码为 0"),
        ]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::GoalTurnStarted { turn: 2, reason, .. }
            if reason.contains("任务文件仍存在")
    )));
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, .. }) if status == "completed"
    ));
    assert_eq!(runner.calls.lock().expect("calls").len(), 5);
}

#[tokio::test]
async fn goal_mode_never_finishes_without_validation_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-no-evidence";
    let runner = GoalSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Ok("claimed done without deleting task".to_string()),
            Ok(r#"{"achieved":true,"reason":"模型声称完成"}"#.to_string()),
        ],
        PlanFixture {
            plan_dir_rel: format!(".plans/task-{task_id}"),
            tasks: vec![entry("1", "验证目标", "1-verify.md", 1, vec![])],
        },
        false,
        None,
    );
    let request = TaskPlanRunRequest {
        query: "没有验证证据不得完成".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Goal,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        let stop = matches!(event, TaskPlanRunStreamEventDto::GoalEvaluated { .. });
        captured.lock().expect("events").push(event);
        !stop
    })
    .await
    .expect("cancel after first evaluation");

    let events = events.lock().expect("events");
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::GoalEvaluated { achieved: false, reason, .. }
            if reason.contains("任务文件")
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::Finished { .. }
    )));
}

#[tokio::test]
async fn goal_mode_blocks_on_explicit_external_blocker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-blocked";
    let runner = GoalSessionRunner::new(
        vec![Ok("plan ready".to_string()), Ok("需要外部凭据".to_string())],
        PlanFixture {
            plan_dir_rel: format!(".plans/task-{task_id}"),
            tasks: vec![entry("1", "外部系统", "1-external.md", 1, vec![])],
        },
        false,
        Some("缺少部署凭据"),
    );
    let request = TaskPlanRunRequest {
        query: "完成外部部署".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Goal,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        captured.lock().expect("events").push(event);
        true
    })
    .await
    .expect("blocked is represented by event");

    let events = events.lock().expect("events");
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Blocked { reason, .. })
            if reason.contains("缺少部署凭据")
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::Finished { .. }
    )));
}

#[tokio::test]
async fn goal_mode_blocks_after_three_identical_infrastructure_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-infrastructure";
    let runner = GoalSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Ok("implementation attempted".to_string()),
            Err("gateway timeout".to_string()),
            Ok("目标续轮重试".to_string()),
            Err("gateway timeout".to_string()),
            Ok("目标续轮再次重试".to_string()),
            Err("gateway timeout".to_string()),
        ],
        PlanFixture {
            plan_dir_rel: format!(".plans/task-{task_id}"),
            tasks: vec![entry("1", "基础设施重试", "1-infra.md", 1, vec![])],
        },
        false,
        None,
    );
    let request = TaskPlanRunRequest {
        query: "验证重复基础设施错误保护".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
        mode: vw_api_types::task::TaskPlanRunMode::Goal,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        captured.lock().expect("events").push(event);
        true
    })
    .await
    .expect("infrastructure failures are represented by event");

    let events = events.lock().expect("events");
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Blocked { reason, .. })
            if reason.contains("连续三轮") && reason.contains("gateway timeout")
    ));
    assert_eq!(runner.calls.lock().expect("calls").len(), 7);
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
    let options = super::gateway_session_options(&request);
    assert_eq!(
        options.get("full_access").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("cwd").and_then(serde_json::Value::as_str),
        Some("/tmp/project")
    );
    assert_eq!(
        options.get("agent").and_then(serde_json::Value::as_str),
        Some("main")
    );
    assert_eq!(
        options.get("acp_test").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert!(!options.contains_key("acp_permission_mode"));
}

#[test]
fn gateway_session_options_use_approve_all_for_acp() {
    let request = SessionRunRequest {
        session_id: "s1".to_string(),
        query: "q".to_string(),
        root: "/tmp/project".to_string(),
        model: None,
        agent: None,
        acp_agent: Some("codex".to_string()),
    };
    let options = super::gateway_session_options(&request);
    assert_eq!(
        options.get("full_access").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("acp_agent").and_then(serde_json::Value::as_str),
        Some("codex")
    );
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-all")
    );
    assert_eq!(
        options.get("acp_test").and_then(serde_json::Value::as_bool),
        Some(true)
    );
}
