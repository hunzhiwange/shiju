//! Merge process-local active runs with persisted history for this workspace.

use crate::{CRON_HISTORY_LIMIT, active_runs, list_history, list_jobs};
use agent_config::runtime::Config;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Serialize)]
pub struct HistorySnapshot {
    pub runs: Vec<HistoryEntry>,
    pub finished_limit: usize,
    pub truncated: bool,
}

#[derive(Serialize)]
pub struct HistoryEntry {
    pub id: Option<i64>,
    pub execution_id: Option<String>,
    pub job_id: String,
    pub name: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub output: Option<String>,
}

pub fn snapshot(config: &Config) -> anyhow::Result<HistorySnapshot> {
    // Capture active runs first so a run that finishes during the query appears only once.
    let active = active_runs::snapshot(config);
    let mut history = list_history(config)?;
    let jobs: HashSet<_> = list_jobs(config)?.into_iter().map(|job| job.id).collect();
    let truncated = history.len() > CRON_HISTORY_LIMIT;
    let completed: HashSet<_> =
        history.iter().map(|entry| (entry.run.job_id.as_str(), entry.run.started_at)).collect();
    let active = active
        .into_iter()
        .filter(|run| {
            jobs.contains(&run.job_id)
                && !completed.contains(&(run.job_id.as_str(), run.started_at))
        })
        .map(|run| {
            (
                run.started_at,
                HistoryEntry {
                    id: None,
                    execution_id: Some(run.execution_id),
                    job_id: run.job_id,
                    name: run.name,
                    status: "running".into(),
                    started_at: run.started_at.to_rfc3339(),
                    finished_at: None,
                    duration_ms: None,
                    output: None,
                },
            )
        })
        .collect::<Vec<_>>();
    history.truncate(CRON_HISTORY_LIMIT);
    let mut rows: Vec<_> = history
        .into_iter()
        .map(|entry| {
            let run = entry.run;
            (
                run.started_at,
                HistoryEntry {
                    id: Some(run.id),
                    execution_id: None,
                    job_id: run.job_id,
                    name: entry.name,
                    status: run.status,
                    started_at: run.started_at.to_rfc3339(),
                    finished_at: Some(run.finished_at.to_rfc3339()),
                    duration_ms: run.duration_ms,
                    output: run.output,
                },
            )
        })
        .chain(active)
        .collect();
    rows.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.id.cmp(&a.1.id))
            .then_with(|| b.1.execution_id.cmp(&a.1.execution_id))
    });
    Ok(HistorySnapshot {
        runs: rows.into_iter().map(|(_, entry)| entry).collect(),
        finished_limit: CRON_HISTORY_LIMIT,
        truncated,
    })
}
