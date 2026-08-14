use super::domain::TaskId;
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::task_graph::{
    AgentTaskDependency, TaskGraph, TaskGraphReadiness, TaskGraphTaskSnapshot,
};

#[test]
fn task_graph_rejects_invalid_topology() {
    let task_a = task("task-a", AgentTaskKind::Finite, AgentTaskStatus::Queued);
    let task_b = task("task-b", AgentTaskKind::Finite, AgentTaskStatus::Queued);

    let self_loop = TaskGraph::new(vec![task_a.clone()], vec![dependency("task-a", "task-a")])
        .expect_err("self loop must be rejected");
    assert!(self_loop.to_string().contains("task-a"));

    let duplicate = TaskGraph::new(
        vec![task_a.clone(), task_b.clone()],
        vec![dependency("task-a", "task-b"), dependency("task-a", "task-b")],
    )
    .expect_err("duplicate edge must be rejected");
    assert!(duplicate.to_string().contains("task-a"));
    assert!(duplicate.to_string().contains("task-b"));

    let missing_node =
        TaskGraph::new(vec![task_a.clone()], vec![dependency("task-a", "task-missing")])
            .expect_err("edge endpoint must exist");
    assert!(missing_node.to_string().contains("task-missing"));

    let cycle = TaskGraph::new(
        vec![task_a.clone(), task_b.clone()],
        vec![dependency("task-a", "task-b"), dependency("task-b", "task-a")],
    )
    .expect_err("cycle must be rejected");
    assert!(cycle.to_string().contains("task-a"));

    let non_finite = TaskGraph::new(
        vec![task("task-infinite", AgentTaskKind::Infinite, AgentTaskStatus::Queued)],
        vec![],
    )
    .expect_err("only finite tasks may be graph nodes");
    assert!(non_finite.to_string().contains("task-infinite"));
}

#[test]
fn task_graph_derives_readiness_from_runtime_status() {
    let queued_successor = task("task-successor", AgentTaskKind::Finite, AgentTaskStatus::Queued);
    let dependency_edge = vec![dependency("task-predecessor", "task-successor")];

    let ready = TaskGraph::new(
        vec![
            task("task-predecessor", AgentTaskKind::Finite, AgentTaskStatus::Completed),
            queued_successor.clone(),
        ],
        dependency_edge.clone(),
    )
    .expect("valid graph");
    assert_eq!(
        ready.readiness_for(&task_id("task-successor")).expect("known task"),
        TaskGraphReadiness::Ready
    );

    let waiting = TaskGraph::new(
        vec![
            task("task-predecessor", AgentTaskKind::Finite, AgentTaskStatus::Submitted),
            queued_successor.clone(),
        ],
        dependency_edge.clone(),
    )
    .expect("valid graph");
    assert_eq!(
        waiting.readiness_for(&task_id("task-successor")).expect("known task"),
        TaskGraphReadiness::WaitingDependencies { task_ids: vec![task_id("task-predecessor")] }
    );

    let blocked = TaskGraph::new(
        vec![
            task("task-predecessor", AgentTaskKind::Finite, AgentTaskStatus::Failed),
            queued_successor.clone(),
        ],
        dependency_edge.clone(),
    )
    .expect("valid graph");
    assert_eq!(
        blocked.readiness_for(&task_id("task-successor")).expect("known task"),
        TaskGraphReadiness::BlockedByDependency { task_ids: vec![task_id("task-predecessor")] }
    );

    let retried_predecessor = TaskGraph::new(
        vec![
            task("task-predecessor", AgentTaskKind::Finite, AgentTaskStatus::Queued),
            queued_successor.clone(),
        ],
        dependency_edge,
    )
    .expect("valid graph");
    assert_eq!(
        retried_predecessor.readiness_for(&task_id("task-successor")).expect("known task"),
        TaskGraphReadiness::WaitingDependencies { task_ids: vec![task_id("task-predecessor")] }
    );

    let non_queued = TaskGraph::new(
        vec![
            task("task-predecessor", AgentTaskKind::Finite, AgentTaskStatus::Completed),
            task("task-successor", AgentTaskKind::Finite, AgentTaskStatus::Assigned),
        ],
        vec![dependency("task-predecessor", "task-successor")],
    )
    .expect("valid graph");
    assert_eq!(
        non_queued.readiness_for(&task_id("task-successor")).expect("known task"),
        TaskGraphReadiness::NotQueued
    );
}

fn task(id: &str, kind: AgentTaskKind, status: AgentTaskStatus) -> TaskGraphTaskSnapshot {
    TaskGraphTaskSnapshot::new(task_id(id), kind, status)
}

fn dependency(predecessor: &str, successor: &str) -> AgentTaskDependency {
    AgentTaskDependency::new(task_id(predecessor), task_id(successor))
}

fn task_id(value: &str) -> TaskId {
    TaskId::new(value).expect("valid task id")
}
