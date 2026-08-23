// 验证目标引擎的状态推进、预算约束与终止条件。
use super::*;

use tempfile::TempDir;

fn step(id: &str, status: StepStatus, attempts: u32) -> Step {
    Step { id: id.to_string(), description: format!("Step {id}"), status, result: None, attempts }
}

fn goal(id: &str, status: GoalStatus, priority: GoalPriority, steps: Vec<Step>) -> Goal {
    Goal {
        id: id.to_string(),
        description: format!("Goal {id}"),
        status,
        priority,
        created_at: String::new(),
        updated_at: String::new(),
        steps,
        context: String::new(),
        last_error: None,
    }
}

#[test]
fn new_targets_state_goals_json_under_workspace() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();
    let engine = GoalEngine::new(workspace);

    assert_eq!(engine.state_path, workspace.join("state").join("goals.json"));
}

#[test]
fn goal_state_deserializes_missing_optional_fields_to_defaults() {
    let state: GoalState = serde_json::from_str(
        r#"{
            "goals": [{
                "id": "g1",
                "description": "missing fields",
                "steps": [{
                    "id": "s1",
                    "description": "also missing fields"
                }]
            }]
        }"#,
    )
    .unwrap();

    let goal = &state.goals[0];
    assert_eq!(goal.status, GoalStatus::Pending);
    assert_eq!(goal.priority, GoalPriority::Medium);
    assert!(goal.created_at.is_empty());
    assert!(goal.updated_at.is_empty());
    assert!(goal.context.is_empty());
    assert!(goal.last_error.is_none());

    let step = &goal.steps[0];
    assert_eq!(step.status, StepStatus::Pending);
    assert_eq!(step.attempts, 0);
    assert!(step.result.is_none());
}

#[test]
fn select_next_actionable_prefers_later_higher_priority_goal() {
    let state = GoalState {
        goals: vec![
            goal(
                "low",
                GoalStatus::InProgress,
                GoalPriority::Low,
                vec![step("a", StepStatus::Pending, 0)],
            ),
            goal(
                "critical",
                GoalStatus::InProgress,
                GoalPriority::Critical,
                vec![step("b", StepStatus::Pending, 0)],
            ),
            goal(
                "high",
                GoalStatus::InProgress,
                GoalPriority::High,
                vec![step("c", StepStatus::Pending, 0)],
            ),
        ],
    };

    assert_eq!(GoalEngine::select_next_actionable(&state), Some((1, 0)));
}

#[test]
fn select_next_actionable_skips_pending_steps_at_retry_limit() {
    let state = GoalState {
        goals: vec![goal(
            "g1",
            GoalStatus::InProgress,
            GoalPriority::Critical,
            vec![
                step("exhausted", StepStatus::Pending, GoalEngine::max_step_attempts()),
                step("fresh", StepStatus::Pending, GoalEngine::max_step_attempts() - 1),
            ],
        )],
    };

    assert_eq!(GoalEngine::select_next_actionable(&state), Some((0, 1)));
}

#[test]
fn find_stalled_goals_ignores_non_in_progress_statuses() {
    let state = GoalState {
        goals: vec![
            goal(
                "blocked",
                GoalStatus::Blocked,
                GoalPriority::Critical,
                vec![step("a", StepStatus::Pending, GoalEngine::max_step_attempts())],
            ),
            goal(
                "cancelled",
                GoalStatus::Cancelled,
                GoalPriority::Critical,
                vec![step("b", StepStatus::Completed, 0)],
            ),
            goal(
                "pending",
                GoalStatus::Pending,
                GoalPriority::Critical,
                vec![step("c", StepStatus::Pending, GoalEngine::max_step_attempts())],
            ),
        ],
    };

    assert!(GoalEngine::find_stalled_goals(&state).is_empty());
}

#[test]
fn select_next_actionable_returns_none_when_all_steps_are_exhausted() {
    let state = GoalState {
        goals: vec![
            goal(
                "g1",
                GoalStatus::InProgress,
                GoalPriority::Critical,
                vec![step("a", StepStatus::Pending, GoalEngine::max_step_attempts())],
            ),
            goal(
                "g2",
                GoalStatus::InProgress,
                GoalPriority::High,
                vec![step("b", StepStatus::Blocked, 0)],
            ),
        ],
    };

    assert_eq!(GoalEngine::select_next_actionable(&state), None);
}

#[test]
fn build_step_prompt_omits_completed_steps_and_context_when_absent() {
    let goal = goal(
        "g1",
        GoalStatus::InProgress,
        GoalPriority::Medium,
        vec![step("first", StepStatus::Pending, 0)],
    );

    let prompt = GoalEngine::build_step_prompt(&goal, &goal.steps[0]);

    assert!(prompt.contains("[Goal Loop] Executing step for goal: Goal g1"));
    assert!(prompt.contains("Current step: Step first"));
    assert!(!prompt.contains("Completed steps:"));
    assert!(!prompt.contains("Context so far:"));
    assert!(!prompt.contains("WARNING:"));
}

#[test]
fn build_reflection_prompt_marks_in_progress_step_as_pending() {
    let mut goal = goal(
        "g1",
        GoalStatus::InProgress,
        GoalPriority::Medium,
        vec![step("running", StepStatus::InProgress, 1)],
    );
    goal.steps[0].result = Some("still running".to_string());

    let prompt = GoalEngine::build_reflection_prompt(&goal);

    assert!(prompt.contains("[pending] Step running: still running"));
}

#[tokio::test]
async fn load_state_reads_existing_goal_file() {
    let tmp = TempDir::new().unwrap();
    let state_dir = tmp.path().join("state");
    tokio::fs::create_dir_all(&state_dir).await.unwrap();
    tokio::fs::write(
        state_dir.join("goals.json"),
        r#"{
            "goals": [{
                "id": "g1",
                "description": "loaded goal",
                "status": "in_progress",
                "priority": "critical",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:01:00Z",
                "steps": [{
                    "id": "s1",
                    "description": "loaded step",
                    "status": "failed",
                    "result": "failed once",
                    "attempts": 2
                }],
                "context": "existing context",
                "last_error": "previous error"
            }]
        }"#,
    )
    .await
    .unwrap();
    let engine = GoalEngine::new(tmp.path());

    let state = engine.load_state().await.unwrap();

    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.goals[0].status, GoalStatus::InProgress);
    assert_eq!(state.goals[0].priority, GoalPriority::Critical);
    assert_eq!(state.goals[0].steps[0].status, StepStatus::Failed);
    assert_eq!(state.goals[0].steps[0].result.as_deref(), Some("failed once"));
    assert_eq!(state.goals[0].steps[0].attempts, 2);
    assert_eq!(state.goals[0].last_error.as_deref(), Some("previous error"));
}

#[tokio::test]
async fn load_state_propagates_error_when_goal_path_is_directory() {
    let tmp = TempDir::new().unwrap();
    let goal_path = tmp.path().join("state").join("goals.json");
    tokio::fs::create_dir_all(&goal_path).await.unwrap();
    let engine = GoalEngine::new(tmp.path());

    let error = engine
        .load_state()
        .await
        .expect_err("a directory cannot be loaded as a serialized goal state");

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn save_state_overwrites_existing_goal_file() {
    let tmp = TempDir::new().unwrap();
    let engine = GoalEngine::new(tmp.path());
    let first = GoalState {
        goals: vec![goal(
            "first",
            GoalStatus::InProgress,
            GoalPriority::Low,
            vec![step("a", StepStatus::Pending, 0)],
        )],
    };
    let second = GoalState {
        goals: vec![goal(
            "second",
            GoalStatus::Completed,
            GoalPriority::Critical,
            vec![step("done", StepStatus::Completed, 1)],
        )],
    };

    engine.save_state(&first).await.unwrap();
    engine.save_state(&second).await.unwrap();
    let loaded = engine.load_state().await.unwrap();

    assert_eq!(loaded.goals.len(), 1);
    assert_eq!(loaded.goals[0].id, "second");
    assert_eq!(loaded.goals[0].status, GoalStatus::Completed);
    assert_eq!(loaded.goals[0].priority, GoalPriority::Critical);
}

#[tokio::test]
async fn save_state_creates_state_directory_without_leaving_tmp_file() {
    let tmp = TempDir::new().unwrap();
    let engine = GoalEngine::new(tmp.path());
    let state = GoalState {
        goals: vec![goal(
            "g1",
            GoalStatus::Completed,
            GoalPriority::High,
            vec![step("done", StepStatus::Completed, 1)],
        )],
    };

    engine.save_state(&state).await.unwrap();

    assert!(tmp.path().join("state").join("goals.json").exists());
    assert!(!tmp.path().join("state").join("goals.json.tmp").exists());
}
