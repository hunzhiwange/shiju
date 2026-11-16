//! 成功事件顺序与原工作流内重试的领域契约。
use crate::{PlanTaskEntry, PlanTasksDocument, SessionRunRequest, SessionRunner};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

#[derive(Clone)]
struct PlanFixture { plan_dir: PathBuf, tasks: Vec<PlanTaskEntry> }
struct ScriptedSessionRunner {
    responses: Mutex<VecDeque<Result<String, String>>>,
    plan_fixture: Mutex<Option<PlanFixture>>,
    task_files: Mutex<Vec<PathBuf>>,
    calls: Mutex<Vec<SessionRunRequest>>,
}
impl ScriptedSessionRunner {
    fn new(responses: Vec<Result<String, String>>, fixture: PlanFixture) -> Self {
        Self { responses: Mutex::new(responses.into()), plan_fixture: Mutex::new(Some(fixture)), task_files: Mutex::new(Vec::new()), calls: Mutex::new(Vec::new()) }
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
        if response.is_ok()
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
    PlanFixture { plan_dir: crate::plan_dir_abs(home, task_id), tasks }
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
    crate::test_host::install();
    let home = PathBuf::from(&request.root);
    crate::runner::run_task_plan_in_home(task_id, request, runner, &home, on_event).await
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

fn plan_dir_exists(home: &Path, task_id: &str) -> bool {
    crate::plan_dir_abs(home, task_id).join("tasks.json").is_file()
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
            && plan_dir == &crate::plan_dir_abs(temp.path(), task_id).to_string_lossy()
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
            .contains(&crate::plan_dir_abs(temp.path(), task_id).to_string_lossy().to_string())
    );
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

