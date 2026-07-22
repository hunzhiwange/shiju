use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamDagProjectStatus {
    Active,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamDagStatus {
    Pending,
    Delegated,
    Completed,
    Blocked,
    Revision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamDagNode {
    pub id: String,
    pub title: String,
    pub assigned_to: Option<String>,
    pub depends_on: Vec<String>,
    pub status: TeamDagStatus,
}

pub fn ready_nodes(
    nodes: &[TeamDagNode],
    project_status: TeamDagProjectStatus,
) -> Result<Vec<TeamDagNode>> {
    if project_status == TeamDagProjectStatus::Paused {
        return Ok(Vec::new());
    }

    validate_dag(nodes)?;
    let completed = nodes
        .iter()
        .filter(|node| node.status == TeamDagStatus::Completed)
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();

    Ok(nodes
        .iter()
        .filter(|node| {
            node.status == TeamDagStatus::Pending
                && node.depends_on.iter().all(|dependency| completed.contains(dependency.as_str()))
        })
        .cloned()
        .collect())
}

pub fn validate_dag(nodes: &[TeamDagNode]) -> Result<()> {
    let mut incoming = BTreeMap::new();
    let mut outgoing = BTreeMap::new();

    for node in nodes {
        if node.id.trim().is_empty() {
            bail!("team DAG node id is empty");
        }
        if incoming.insert(node.id.clone(), node.depends_on.len()).is_some() {
            bail!("duplicate team DAG node id: {}", node.id);
        }
        outgoing.insert(node.id.clone(), Vec::new());
    }

    for node in nodes {
        for dependency in &node.depends_on {
            let Some(children) = outgoing.get_mut(dependency.as_str()) else {
                bail!("team DAG node {} depends on unknown node: {}", node.id, dependency);
            };
            children.push(node.id.clone());
        }
    }

    let mut queue = incoming
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
        .collect::<VecDeque<_>>();
    let mut visited = 0;

    while let Some(node_id) = queue.pop_front() {
        visited += 1;
        for child_id in outgoing.get(node_id.as_str()).cloned().unwrap_or_default() {
            let Some(count) = incoming.get_mut(child_id.as_str()) else {
                bail!("team DAG node {} depends on unknown node: {}", child_id, node_id);
            };
            *count -= 1;
            if *count == 0 {
                queue.push_back(child_id);
            }
        }
    }

    if visited != nodes.len() {
        bail!("cycle detected in team DAG");
    }

    Ok(())
}
