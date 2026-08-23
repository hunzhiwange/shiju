//! 目标引擎，维护持久化目标、步骤状态、重试计数与提示上下文。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Maximum retry attempts per step before marking the goal as blocked.
const MAX_STEP_ATTEMPTS: u32 = 3;

// ── Data Structures ─────────────────────────────────────────────

/// Root state persisted to `{workspace}/state/goals.json`.
/// Format matches the `goal-tracker` skill's file layout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalState {
    #[serde(default)]
    pub goals: Vec<Goal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub status: GoalStatus,
    #[serde(default)]
    pub priority: GoalPriority,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    /// Accumulated context from previous step results.
    #[serde(default)]
    pub context: String,
    /// Last error encountered during step execution.
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

impl<'de> Deserialize<'de> for GoalStatus {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "blocked" => Self::Blocked,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GoalPriority {
    Low = 0,
    #[default]
    Medium = 1,
    High = 2,
    Critical = 3,
}

impl PartialOrd for GoalPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GoalPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub status: StepStatus,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

impl<'de> Deserialize<'de> for StepStatus {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "blocked" => Self::Blocked,
            _ => Self::Pending,
        })
    }
}

// ── GoalEngine ──────────────────────────────────────────────────

pub struct GoalEngine {
    state_path: PathBuf,
}

impl GoalEngine {
    pub fn new(workspace_dir: &Path) -> Self {
        Self { state_path: workspace_dir.join("state").join("goals.json") }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn load_state(&self) -> Result<GoalState> {
        Ok(GoalState::default())
    }

    /// Load goal state from disk. Returns empty state if file doesn't exist.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_state(&self) -> Result<GoalState> {
        if !self.state_path.exists() {
            return Ok(GoalState::default());
        }
        let bytes = tokio::fs::read(&self.state_path).await?;
        if bytes.is_empty() {
            return Ok(GoalState::default());
        }
        let state: GoalState = serde_json::from_slice(&bytes)?;
        Ok(state)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn save_state(&self, _state: &GoalState) -> Result<()> {
        Ok(())
    }

    /// Atomic save: write to .tmp then rename.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn save_state(&self, state: &GoalState) -> Result<()> {
        if let Some(parent) = self.state_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let tmp = self.state_path.with_extension("json.tmp");
        let data = serde_json::to_vec_pretty(state)?;
        tokio::fs::write(&tmp, data).await?;
        tokio::fs::rename(&tmp, &self.state_path).await?;
        Ok(())
    }

    /// Select the next actionable (goal_index, step_index) pair.
    ///
    /// Strategy: highest-priority in-progress goal, first pending step
    /// that hasn't exceeded `MAX_STEP_ATTEMPTS`.
    pub fn select_next_actionable(state: &GoalState) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, GoalPriority)> = None;

        for (gi, goal) in state.goals.iter().enumerate() {
            if goal.status != GoalStatus::InProgress {
                continue;
            }
            if let Some(si) = goal
                .steps
                .iter()
                .position(|s| s.status == StepStatus::Pending && s.attempts < MAX_STEP_ATTEMPTS)
            {
                match best {
                    Some((_, _, ref bp)) if goal.priority <= *bp => {}
                    _ => best = Some((gi, si, goal.priority)),
                }
            }
        }

        best.map(|(gi, si, _)| (gi, si))
    }

    /// Build a focused prompt for the agent to execute one step.
    pub fn build_step_prompt(goal: &Goal, step: &Step) -> String {
        let mut prompt = String::new();

        let _ = writeln!(prompt, "[Goal Loop] Executing step for goal: {}\n", goal.description);

        // Completed steps summary
        let completed: Vec<&Step> =
            goal.steps.iter().filter(|s| s.status == StepStatus::Completed).collect();
        if !completed.is_empty() {
            prompt.push_str("Completed steps:\n");
            for s in &completed {
                let _ = writeln!(
                    prompt,
                    "- [done] {}: {}",
                    s.description,
                    s.result.as_deref().unwrap_or("(no result)")
                );
            }
            prompt.push('\n');
        }

        // Accumulated context
        if !goal.context.is_empty() {
            let _ = write!(prompt, "Context so far:\n{}\n\n", goal.context);
        }

        // Current step
        let _ = write!(
            prompt,
            "Current step: {}\n\
             Please execute this step. Provide a clear summary of what you did and the outcome.\n",
            step.description
        );

        // Retry warning
        if step.attempts > 0 {
            let _ = write!(
                prompt,
                "\nWARNING: This step has failed {} time(s) before. \
                 Last error: {}\n\
                 Try a different approach.\n",
                step.attempts,
                goal.last_error.as_deref().unwrap_or("unknown")
            );
        }

        prompt
    }

    /// Simple heuristic: output containing error indicators → failure.
    pub fn interpret_result(output: &str) -> bool {
        let lower = output.to_ascii_lowercase();
        let failure_indicators =
            ["failed to", "error:", "unable to", "cannot ", "could not", "fatal:", "panic:"];
        !failure_indicators.iter().any(|ind| lower.contains(ind))
    }

    pub fn max_step_attempts() -> u32 {
        MAX_STEP_ATTEMPTS
    }

    /// Find in-progress goals that have no actionable steps remaining.
    ///
    /// A goal is "stalled" when it is `InProgress` but every step is either
    /// completed, blocked, or has exhausted its retry attempts. These goals
    /// need a reflection session to decide: add new steps, mark completed,
    /// mark blocked, or escalate to the user.
    pub fn find_stalled_goals(state: &GoalState) -> Vec<usize> {
        state
            .goals
            .iter()
            .enumerate()
            .filter(|(_, g)| g.status == GoalStatus::InProgress)
            .filter(|(_, g)| {
                !g.steps.is_empty()
                    && !g
                        .steps
                        .iter()
                        .any(|s| s.status == StepStatus::Pending && s.attempts < MAX_STEP_ATTEMPTS)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Build a reflection prompt for a stalled goal.
    ///
    /// The agent is asked to review the goal's overall progress and decide
    /// what to do next: add new steps, mark the goal completed, or escalate.
    pub fn build_reflection_prompt(goal: &Goal) -> String {
        let mut prompt = String::new();

        let _ = writeln!(prompt, "[Goal Reflection] Goal: {}\n", goal.description);

        prompt.push_str("All steps have been attempted. Here is the current state:\n\n");

        for s in &goal.steps {
            let status_tag = match s.status {
                StepStatus::Completed => "done",
                StepStatus::Failed | StepStatus::Blocked => "blocked",
                _ if s.attempts >= MAX_STEP_ATTEMPTS => "exhausted",
                _ => "pending",
            };
            let result = s.result.as_deref().unwrap_or("(no result)");
            let _ = writeln!(prompt, "- [{status_tag}] {}: {result}", s.description);
        }

        if !goal.context.is_empty() {
            let _ = write!(prompt, "\nAccumulated context:\n{}\n", goal.context);
        }

        if let Some(ref err) = goal.last_error {
            let _ = write!(prompt, "\nLast error: {err}\n");
        }

        prompt.push_str(
            "\nReflect on this goal and take ONE of the following actions:\n\
             1. If the goal is effectively achieved, update state/goals.json to mark it `completed`.\n\
             2. If some steps failed but you can try a different approach, add NEW steps to \
                state/goals.json with fresh descriptions (don't reuse failed step IDs).\n\
             3. If the goal is truly blocked and needs human input, mark it `blocked` in \
                state/goals.json and explain what you need from the user.\n\
             4. Use memory_store to record what you learned from the failures.\n\n\
             Be decisive. Do not leave the goal in its current state.",
        );

        prompt
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
