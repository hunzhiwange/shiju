//! `run_task_plan` 注入式 SessionRunner 行为测试。

use super::{
    PlanTaskEntry, PlanTasksDocument, SessionRunRequest, SessionRunner,
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
    task_files: Arc<Mutex<Vec<PathBuf>>>,
    /// 子任务完成后写入完整任务清单证据，便于目标模式按确定性清单完成。
    complete_goal_on_evaluation: bool,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
}

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

#[derive(Clone)]
struct GoalSessionRunner {
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    task_files: Arc<Mutex<Vec<PathBuf>>>,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
    initial_blocker: Option<String>,
}

#[derive(Clone)]
struct DescendantGatedSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    plan_dir: PathBuf,
    descendant_started: Arc<Semaphore>,
    descendant_release: Arc<Semaphore>,
}

#[derive(Clone)]
struct CancellationGatedSessionRunner {
    started: Arc<Semaphore>,
}

#[derive(Clone)]
struct PromptPathSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    canonical_plan_dir: PathBuf,
    canonical_task_file: PathBuf,
    decoy_task_file: PathBuf,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
}

#[derive(Clone)]
struct ContinuationEvidenceSessionRunner {
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
    canonical_plan_dir: PathBuf,
    canonical_task_file: PathBuf,
    decoy_task_file: PathBuf,
    prior_output: String,
    calls: Arc<Mutex<Vec<SessionRunRequest>>>,
}

impl Default for CancellationGatedSessionRunner {
    fn default() -> Self {
        Self { started: Arc::new(Semaphore::new(0)) }
    }
}

impl PromptPathSessionRunner {
    fn new(plan_fixture: PlanFixture, canonical_task_file: PathBuf, decoy_task_file: PathBuf) -> Self {
        Self {
            canonical_plan_dir: plan_fixture.plan_dir.clone(),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            canonical_task_file,
            decoy_task_file,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ContinuationEvidenceSessionRunner {
    fn new(
        plan_fixture: PlanFixture,
        canonical_task_file: PathBuf,
        decoy_task_file: PathBuf,
        prior_output: &str,
    ) -> Self {
        Self {
            canonical_plan_dir: plan_fixture.plan_dir.clone(),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            canonical_task_file,
            decoy_task_file,
            prior_output: prior_output.to_string(),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ScriptedSessionRunner {
    fn new(responses: Vec<Result<String, String>>, plan_fixture: PlanFixture) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            task_files: Arc::new(Mutex::new(Vec::new())),
            complete_goal_on_evaluation: true,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn without_plan_files(responses: Vec<Result<String, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(None)),
            task_files: Arc::new(Mutex::new(Vec::new())),
            complete_goal_on_evaluation: false,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
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

impl GoalSessionRunner {
    fn new(
        responses: Vec<Result<String, String>>,
        plan_fixture: PlanFixture,
        initial_blocker: Option<&str>,
    ) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
            task_files: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            initial_blocker: initial_blocker.map(str::to_string),
        }
    }
}

impl DescendantGatedSessionRunner {
    fn new(plan_fixture: PlanFixture) -> Self {
        Self {
            plan_dir: plan_fixture.plan_dir.clone(),
            plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),
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
impl SessionRunner for PromptPathSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        self.calls.lock().expect("calls lock").push(request.clone());
        if let Some(fixture) = self.plan_fixture.lock().expect("fixture lock").take() {
            write_plan_fixture(&request.root, &fixture)?;
            return Ok("plan ready".to_string());
        }

        let names_canonical_paths = request
            .query
            .contains(&self.canonical_plan_dir.to_string_lossy().to_string())
            && request
                .query
                .contains(&self.canonical_task_file.to_string_lossy().to_string());
        let target =
            if names_canonical_paths { &self.canonical_task_file } else { &self.decoy_task_file };
        std::fs::write(
            target,
            "# 任务\n\n状态：已完成\n\n完成条件：路径绑定已验证\n\n验证命令：cargo test\n\n验证结果：退出码：0；通过\n",
        )
        .map_err(|error| format!("write selected task failed: {error}"))?;
        Ok("subtask done".to_string())
    }
}

#[async_trait]
impl SessionRunner for ContinuationEvidenceSessionRunner {
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
            return Ok("plan ready".to_string());
        }
        if call_index == 2 {
            return Ok(self.prior_output.clone());
        }

        let receives_required_context = request.query.contains(&self.prior_output)
            && request
                .query
                .contains(&self.canonical_plan_dir.to_string_lossy().to_string())
            && request
                .query
                .contains(&self.canonical_task_file.to_string_lossy().to_string())
            && request.query.contains("状态不是");
        let target = if receives_required_context {
            &self.canonical_task_file
        } else {
            &self.decoy_task_file
        };
        std::fs::write(
            target,
            "# 任务\n\n状态：已完成\n\n完成条件：续轮复用证据完成修正\n\n验证命令：cargo test\n\n验证结果：退出码：0；通过\n",
        )
        .map_err(|error| format!("write selected continuation task failed: {error}"))?;
        Ok("目标续轮已修正任务文件".to_string())
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
            let plan_abs = fixture.plan_dir.clone();
            *self.task_files.lock().expect("task files lock") =
                fixture.tasks.iter().map(|task| plan_abs.join(&task.file)).collect();
        }

        let response = self
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err("scripted session runner: no more responses".to_string()));

        // 子任务结束后写入完成清单，使固定目标模式无需模型验收即可完成。
        if self.complete_goal_on_evaluation
            && response.is_ok()
            && !request.session_id.ends_with("-plan")
        {
            for task_file in self.task_files.lock().expect("task files lock").iter() {
                if task_file.exists() {
                    std::fs::write(
                        task_file,
                        "# 任务\n\n状态：已完成\n\n完成条件：子任务已实现\n\n验证命令：cargo test -p vw-agent --lib\n\n验证结果：退出码：0；通过\n",
                    )
                    .map_err(|error| format!("write completed task failed: {error}"))?;
                }
            }
        }

        response
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
            let plan_abs = fixture.plan_dir.clone();
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
            let fixture =
                self.plan_fixture.lock().expect("fixture lock").take().expect("planning fixture");
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
            .mark_succeeded(&request.session_id, &child_session_id, "delegated verification")
            .map_err(|error| error.to_string())?;
        coordinator
            .register(&child_session_id, &descendant_session_id, "reviewer", "verify parent output")
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

        complete_task_mds(&self.plan_dir);
        Ok("parent session returned".to_string())
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
        | TaskPlanRunStreamEventDto::SubtaskFinished { subtask_id, .. } => {
            Some(subtask_id.as_str())
        }
        TaskPlanRunStreamEventDto::Blocked { subtask_id: Some(subtask_id), .. } => {
            Some(subtask_id.as_str())
        }
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

fn plan_dir_exists(home: &Path, task_id: &str) -> bool {
    super::plan_dir_abs(home, task_id).join("tasks.json").is_file()
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
    let runner = DescendantGatedSessionRunner::new(plan_fixture(
        temp.path(),
        task_id,
        vec![
            entry("1", "父任务一", "1-parent.md", 1, vec![]),
            entry("2", "父任务二", "2-parent.md", 1, vec![]),
            entry("3", "父任务三", "3-parent.md", 1, vec![]),
        ],
    ));
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
        };
        run_task_plan(task_id, &request, &runner, |event| {
            collected_events.lock().expect("events lock").push(event);
            true
        })
        .await
    });

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        descendant_started
            .acquire_many(3)
            .await
            .expect("descendant start semaphore should remain open")
            .forget();
    })
    .await
    .expect("same-batch nested descendants should start");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    assert!(!run.is_finished(), "plan-run must wait while a nested descendant remains active");
    assert!(
        !events
            .lock()
            .expect("events lock")
            .iter()
            .any(|event| matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. })),
        "SubtaskFinished must not be emitted before nested descendants are terminal"
    );

    descendant_release.add_permits(3);
    tokio::time::timeout(std::time::Duration::from_secs(5), run)
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
            TaskPlanRunStreamEventDto::SessionWaiting { active_descendant_session_ids, .. } => {
                Some((index, active_descendant_session_ids))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(waiting.len(), 3);
    assert!(waiting.iter().all(|(index, _)| *index < first_finished));
    assert!(waiting.iter().all(|(_, active)| active.len() == 1));
    assert!(waiting.iter().all(|(_, active)| active[0].ends_with("-nested")));

    let finished = events
        .iter()
        .filter_map(|event| match event {
            TaskPlanRunStreamEventDto::SubtaskFinished {
                subtask_id, output: Some(output), ..
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
async fn run_plan_emits_full_success_event_sequence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-success";

    let fixture = plan_fixture(
        temp.path(),
        task_id,
        vec![
            entry("1", "第一批 A", "1-a.md", 1, vec![]),
            entry("2", "第一批 B", "2-b.md", 1, vec![]),
            entry("3", "第二批", "3-c.md", 2, vec![1]),
        ],
    );

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Ok("subtask 1 ok".to_string()),
            Ok("subtask 2 ok".to_string()),
            Ok("subtask 3 ok".to_string()),
            Ok(r#"{"achieved":true,"reason":"验证通过"}"#.to_string()),
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
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await
    .expect("run_task_plan should succeed");

    assert!(plan_dir_exists(temp.path(), task_id));

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
            plan_dir,
            session_id,
            ..
        } if id == task_id
            && plan_dir == &super::plan_dir_abs(temp.path(), task_id).to_string_lossy()
            && session_id == "demo-success-plan"
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
        }) if status == "completed" && session_id.starts_with("demo-success-goal")
    ));

    let calls = runner.calls.lock().expect("calls").clone();
    assert_eq!(calls.len(), 4, "1 planning + 3 subtasks");
    assert_eq!(calls[0].session_id, "demo-success-plan");
    assert!(calls.iter().all(|call| call.root == root));
    assert!(
        calls[0]
            .query
            .contains(&super::plan_dir_abs(temp.path(), task_id).to_string_lossy().to_string())
    );
}

#[tokio::test]
async fn run_plan_ignores_legacy_project_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let project_root = temp.path().join("project");
    let task_id = "legacy-conflict";
    let legacy_dir = project_root.join(".plans").join(format!("task-{task_id}"));
    std::fs::create_dir_all(&legacy_dir).expect("create legacy plan dir");
    std::fs::write(legacy_dir.join("legacy-only.txt"), "must stay isolated")
        .expect("write legacy marker");

    let plan_abs = super::plan_dir_abs(&home, task_id);
    let runner = ScriptedSessionRunner::new(
        vec![Ok("plan ready".to_string()), Ok("subtask done".to_string())],
        PlanFixture {
            plan_dir: plan_abs.clone(),
            tasks: vec![entry("1", "实现新路径", "1-storage.md", 1, vec![])],
        },
    );
    let request = TaskPlanRunRequest {
        query: "使用用户级计划缓存".to_string(),
        root: project_root.to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };

    super::runner::run_task_plan_in_home(task_id, &request, &runner, &home, |_| true)
        .await
        .expect("run_task_plan should use the injected home");

    assert!(plan_abs.join("tasks.json").is_file());
    assert!(!plan_abs.join("legacy-only.txt").exists());
    assert_eq!(
        std::fs::read_to_string(legacy_dir.join("legacy-only.txt")).expect("read legacy marker"),
        "must stay isolated"
    );
    let calls = runner.calls.lock().expect("calls lock");
    assert!(calls.iter().all(|call| call.root == request.root));
    assert!(calls[0].query.contains(&plan_abs.to_string_lossy().to_string()));
}

#[tokio::test]
async fn task_plan_run_updates_only_canonical_user_plan_task_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("runtime-home");
    let project_root = temp.path().join("project");
    let task_id = "dynamic/path.id";
    let task_file_name = "1-bind-path.md";
    let canonical_plan_dir = super::plan_dir_abs(&home, task_id);
    let canonical_task_file = canonical_plan_dir.join(task_file_name);
    let decoy_plan_dir =
        project_root.join(".plans").join(super::prompts::plan_dir_name(task_id));
    let decoy_task_file = decoy_plan_dir.join(task_file_name);
    std::fs::create_dir_all(&decoy_plan_dir).expect("create decoy plan dir");
    let decoy_before = "# 诱饵任务\n\n状态：未开始\n";
    std::fs::write(&decoy_task_file, decoy_before).expect("write decoy task");

    let runner = PromptPathSessionRunner::new(
        PlanFixture {
            plan_dir: canonical_plan_dir.clone(),
            tasks: vec![entry("1", "绑定路径", task_file_name, 1, vec![])],
        },
        canonical_task_file.clone(),
        decoy_task_file.clone(),
    );
    let request = TaskPlanRunRequest {
        query: "只更新规范任务文件".to_string(),
        root: project_root.to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };

    super::runner::run_task_plan_in_home(task_id, &request, &runner, &home, |event| {
        !matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. })
    })
    .await
    .expect("plan run should reach subtask completion");

    assert_eq!(
        std::fs::read_to_string(&canonical_task_file).expect("read canonical task"),
        "# 任务\n\n状态：已完成\n\n完成条件：路径绑定已验证\n\n验证命令：cargo test\n\n验证结果：退出码：0；通过\n"
    );
    assert_eq!(
        std::fs::read_to_string(&decoy_task_file).expect("read decoy task"),
        decoy_before
    );
    let calls = runner.calls.lock().expect("calls lock");
    let execution_prompt = &calls[1].query;
    assert!(execution_prompt.contains(&canonical_plan_dir.to_string_lossy().to_string()));
    assert!(execution_prompt.contains(&canonical_task_file.to_string_lossy().to_string()));
    assert!(!execution_prompt.contains(&decoy_task_file.to_string_lossy().to_string()));
}

#[tokio::test]
async fn run_plan_reports_missing_home_and_creation_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    let request = TaskPlanRunRequest {
        query: "生成计划".to_string(),
        root: temp.path().to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let runner = ScriptedSessionRunner::without_plan_files(Vec::new());

    let missing_home_error = super::runner::run_task_plan_in_home(
        "missing-home",
        &request,
        &runner,
        Path::new(""),
        |_| true,
    )
    .await
    .expect_err("empty home must fail explicitly");
    assert!(missing_home_error.contains("无法解析用户 home"));
    assert!(missing_home_error.contains("task-missing-home"));

    let home_file = temp.path().join("home-file");
    std::fs::write(&home_file, "not a directory").expect("write home blocker");
    let target = super::plan_dir_abs(&home_file, "create-failure");
    let creation_error = super::runner::run_task_plan_in_home(
        "create-failure",
        &request,
        &runner,
        &home_file,
        |_| true,
    )
    .await
    .expect_err("non-directory home must fail directory creation");
    assert!(creation_error.contains(&target.to_string_lossy().to_string()));
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

#[tokio::test]
async fn run_plan_retries_failed_subtask_without_replanning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-recovered";

    let fixture = plan_fixture(
        temp.path(),
        task_id,
        vec![
            entry("1", "重试后恢复", "1-recover.md", 1, vec![]),
            entry("2", "继续后续批次", "2-later.md", 2, vec![1]),
        ],
    );

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Err("first transient failure".to_string()),
            Err("second transient failure".to_string()),
            Ok("subtask recovered".to_string()),
            Ok("later batch completed".to_string()),
            Ok(r#"{"achieved":true,"reason":"验证通过"}"#.to_string()),
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
    assert!(events.iter().all(|event| !matches!(event, TaskPlanRunStreamEventDto::Blocked { .. })));
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, .. }) if status == "completed"
    ));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, TaskPlanRunStreamEventDto::PlanningStarted { .. }))
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
    assert_eq!(calls.iter().filter(|call| call.session_id == "demo-recovered-plan").count(), 1);
    assert_eq!(calls.iter().filter(|call| call.session_id == "demo-recovered-1").count(), 3);
    assert!(
        calls.iter().any(|call| call.session_id == "demo-recovered-2"),
        "later batch must continue after retry succeeds"
    );
}

#[tokio::test]
async fn goal_mode_continues_after_subtask_retries_and_finishes_via_checklist() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-blocked";

    let fixture = plan_fixture(
        temp.path(),
        task_id,
        vec![
            entry("1", "会失败", "1-fail.md", 1, vec![]),
            entry("2", "后续批次可继续", "2-later.md", 2, vec![1]),
        ],
    );

    let runner = ScriptedSessionRunner::new(
        vec![
            Ok("plan ready".to_string()),
            Err("subtask boom 1".to_string()),
            Err("subtask boom 2".to_string()),
            Err("subtask boom 3".to_string()),
            Err("subtask boom 4".to_string()),
            Ok("subtask 2 recovered".to_string()),
        ],
        fixture,
    );

    let request = TaskPlanRunRequest {
        query: "目标模式在子任务重试耗尽后继续后续批次".to_string(),
        root: root.clone(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_cb = Arc::clone(&events);
    let result = run_task_plan(task_id, &request, &runner, |event| {
        events_for_cb.lock().expect("events").push(event);
        true
    })
    .await;

    assert!(result.is_ok(), "goal path returns Ok after finishing");

    let events = events.lock().expect("events").clone();
    let kinds: Vec<_> = events
        .iter()
        .filter(|event| !matches!(event, TaskPlanRunStreamEventDto::SessionDelta { .. }))
        .map(event_kind)
        .collect();

    assert_eq!(kinds.first().copied(), Some("planning_started"));
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::BatchFinished { batch: 1, success: false, .. }
    )));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            TaskPlanRunStreamEventDto::SubtaskStarted {
                batch: 2,
                subtask_id,
                ..
            } if subtask_id == "2"
        )
    }));
    assert_eq!(kinds.last().copied(), Some("finished"));

    let calls = runner.calls.lock().expect("calls").clone();
    assert!(
        calls.iter().any(|call| call.session_id == "demo-blocked-2"),
        "goal mode should start subtask 2 after batch 1 retries: {:?}",
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
    assert!(err.contains("没有用工具落盘"), "error should include planning output preview: {err}");

    let events = events.lock().expect("events").clone();
    assert_eq!(event_kind(&events[0]), "planning_started");
    assert_eq!(event_kind(events.last().expect("error event")), "error");
    assert!(super::plan_dir_abs(temp.path(), task_id).is_dir());
}

#[tokio::test]
async fn goal_mode_finishes_from_completed_task_checklist_without_evaluator() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-checklist";
    let runner = ScriptedSessionRunner::new(
        vec![Ok("plan ready".to_string()), Ok("implementation complete".to_string())],
        plan_fixture(temp.path(), task_id, vec![entry("1", "实现目标", "1-goal.md", 1, vec![])]),
    );
    let request = TaskPlanRunRequest {
        query: "完成目标并提供验证证据".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        captured.lock().expect("events").push(event);
        true
    })
    .await
    .expect("completed task checklist finishes workflow");

    let events = events.lock().expect("events");
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, .. }) if status == "completed"
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::GoalEvaluated { .. }
            | TaskPlanRunStreamEventDto::GoalTurnStarted { .. }
    )));
    let calls = runner.calls.lock().expect("calls");
    assert_eq!(calls.len(), 2, "planning + subtask only");
    assert!(calls.iter().all(|call| !call.session_id.contains("-goal-eval-")));
}

#[tokio::test]
async fn subtask_finished_requires_canonical_task_file_completion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-subtask-contract";
    let claimed_output = "子任务输出声称实现与验证均已完成";
    let runner = GoalSessionRunner::new(
        vec![Ok("plan ready".to_string()), Ok(claimed_output.to_string())],
        plan_fixture(
            temp.path(),
            task_id,
            vec![entry("1", "契约验收", "1-contract.md", 1, vec![])],
        ),
        None,
    );
    let request = TaskPlanRunRequest {
        query: "完成目标并严格检查任务文件".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);

    run_task_plan(task_id, &request, &runner, |event| {
        let stop = matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. });
        captured.lock().expect("events").push(event);
        !stop
    })
    .await
    .expect("stop after first subtask completion event");

    let events = events.lock().expect("events");
    let finished = events
        .iter()
        .find(|event| matches!(event, TaskPlanRunStreamEventDto::SubtaskFinished { .. }))
        .expect("subtask finished event");
    assert!(matches!(
        finished,
        TaskPlanRunStreamEventDto::SubtaskFinished {
            success: false,
            output: Some(output),
            error: Some(error),
            ..
        } if output == claimed_output && error.contains("状态不是")
    ));
}

#[tokio::test]
async fn goal_continuation_receives_prior_execution_evidence_and_canonical_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("runtime-home");
    let project_root = temp.path().join("project");
    let task_id = "dynamic/continuation.id";
    let task_file_name = "1-continuation.md";
    let canonical_plan_dir = super::plan_dir_abs(&home, task_id);
    let canonical_task_file = canonical_plan_dir.join(task_file_name);
    let decoy_task_file = project_root
        .join(".plans")
        .join(super::prompts::plan_dir_name(task_id))
        .join(task_file_name);
    std::fs::create_dir_all(decoy_task_file.parent().expect("decoy parent"))
        .expect("create decoy plan dir");
    let decoy_before = "# 诱饵任务\n\n状态：未开始\n";
    std::fs::write(&decoy_task_file, decoy_before).expect("write decoy task");
    let prior_output = "既有执行证据：实现文件已修改，聚焦测试已通过";
    let runner = ContinuationEvidenceSessionRunner::new(
        PlanFixture {
            plan_dir: canonical_plan_dir.clone(),
            tasks: vec![entry("1", "续轮修正", task_file_name, 1, vec![])],
        },
        canonical_task_file.clone(),
        decoy_task_file.clone(),
        prior_output,
    );
    let request = TaskPlanRunRequest {
        query: "目标续轮复用已有证据修正清单".to_string(),
        root: project_root.to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let mut goal_evaluations = 0_u32;

    super::runner::run_task_plan_in_home(task_id, &request, &runner, &home, |event| {
        if matches!(event, TaskPlanRunStreamEventDto::GoalEvaluated { .. }) {
            goal_evaluations += 1;
        }
        let keep_running = goal_evaluations < 2;
        captured.lock().expect("events").push(event);
        keep_running
    })
    .await
    .expect("continuation workflow should run");

    let calls = runner.calls.lock().expect("calls");
    let continuation_prompt = &calls[2].query;
    assert!(continuation_prompt.contains(prior_output));
    assert!(continuation_prompt.contains(&canonical_plan_dir.to_string_lossy().to_string()));
    assert!(continuation_prompt.contains(&canonical_task_file.to_string_lossy().to_string()));
    assert!(continuation_prompt.contains("状态不是"));
    drop(calls);

    assert_eq!(
        std::fs::read_to_string(&decoy_task_file).expect("read decoy task"),
        decoy_before
    );
    let events = events.lock().expect("events");
    assert!(events.iter().any(|event| matches!(
        event,
        TaskPlanRunStreamEventDto::SubtaskFinished {
            success: false,
            output: Some(output),
            error: Some(error),
            ..
        } if output == prior_output && error.contains("状态不是")
    )));
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Finished { status, .. }) if status == "completed"
    ));
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
        plan_fixture(temp.path(), task_id, vec![entry("1", "验证目标", "1-verify.md", 1, vec![])]),
        None,
    );
    let request = TaskPlanRunRequest {
        query: "没有验证证据不得完成".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
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
            if reason.contains("状态不是")
    )));
    assert!(
        !events.iter().any(|event| matches!(event, TaskPlanRunStreamEventDto::Finished { .. }))
    );
}

#[tokio::test]
async fn goal_mode_blocks_on_explicit_external_blocker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "demo-goal-blocked";
    let runner = GoalSessionRunner::new(
        vec![Ok("plan ready".to_string()), Ok("需要外部凭据".to_string())],
        plan_fixture(
            temp.path(),
            task_id,
            vec![entry("1", "外部系统", "1-external.md", 1, vec![])],
        ),
        Some("缺少部署凭据"),
    );
    let request = TaskPlanRunRequest {
        query: "完成外部部署".to_string(),
        root,
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
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
    assert!(
        !events.iter().any(|event| matches!(event, TaskPlanRunStreamEventDto::Finished { .. }))
    );
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
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("cwd").and_then(serde_json::Value::as_str), Some("/tmp/project"));
    assert_eq!(options.get("agent").and_then(serde_json::Value::as_str), Some("main"));
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(options.get("goal_mode").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("workflow_mode").and_then(serde_json::Value::as_bool), Some(true));
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
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("acp_agent").and_then(serde_json::Value::as_str), Some("codex"));
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-all")
    );
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(true));
}

#[test]
fn gateway_plan_run_preallocates_assistant_message_id() {
    let first = super::runner::preallocate_assistant_message_id()
        .expect("first assistant message id");
    let second = super::runner::preallocate_assistant_message_id()
        .expect("second assistant message id");

    assert!(!first.trim().is_empty());
    assert!(!second.trim().is_empty());
    assert_ne!(first, second);
}
