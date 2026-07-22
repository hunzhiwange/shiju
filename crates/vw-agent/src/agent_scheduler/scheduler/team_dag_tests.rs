use anyhow::Result;

use super::team_dag::{TeamDagNode, TeamDagProjectStatus, TeamDagStatus, ready_nodes};

#[test]
fn agent_team_dag_returns_ready_nodes_and_rejects_invalid_graph() -> Result<()> {
    let nodes = vec![
        node("design", TeamDagStatus::Completed, &[]),
        node("backend", TeamDagStatus::Pending, &["design"]),
        node("frontend", TeamDagStatus::Pending, &["design"]),
        node("integration", TeamDagStatus::Pending, &["backend", "frontend"]),
        node("already-delegated", TeamDagStatus::Delegated, &["design"]),
    ];

    let ready = ready_nodes(&nodes, TeamDagProjectStatus::Active)?;
    let ready_ids = ready.iter().map(|node| node.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ready_ids, vec!["backend", "frontend"]);
    assert!(!ready_ids.contains(&"integration"));

    let paused_ready = ready_nodes(&nodes, TeamDagProjectStatus::Paused)?;
    assert!(paused_ready.is_empty());

    let duplicate_error = ready_nodes(
        &[node("dup", TeamDagStatus::Pending, &[]), node("dup", TeamDagStatus::Pending, &[])],
        TeamDagProjectStatus::Active,
    )
    .unwrap_err()
    .to_string();
    assert!(duplicate_error.contains("duplicate"));

    let unknown_error = ready_nodes(
        &[node("blocked", TeamDagStatus::Pending, &["missing"])],
        TeamDagProjectStatus::Active,
    )
    .unwrap_err()
    .to_string();
    assert!(unknown_error.contains("unknown"));

    let cycle_error = ready_nodes(
        &[
            node("cycle-a", TeamDagStatus::Pending, &["cycle-b"]),
            node("cycle-b", TeamDagStatus::Pending, &["cycle-a"]),
        ],
        TeamDagProjectStatus::Active,
    )
    .unwrap_err()
    .to_string();
    assert!(cycle_error.contains("cycle"));

    let empty_id_error =
        ready_nodes(&[node("   ", TeamDagStatus::Pending, &[])], TeamDagProjectStatus::Active)
            .unwrap_err()
            .to_string();
    assert!(empty_id_error.contains("node id is empty"));

    Ok(())
}

fn node(id: &str, status: TeamDagStatus, depends_on: &[&str]) -> TeamDagNode {
    TeamDagNode {
        id: id.to_string(),
        title: format!("{id} task"),
        assigned_to: Some(format!("{id}-worker")),
        depends_on: depends_on.iter().map(|dependency| dependency.to_string()).collect(),
        status,
    }
}
