//! 夜间记忆归并任务的固定提示、默认计划与安装入口。
//!
//! 归并任务始终使用隔离会话，不投递到主会话，也不默认指定模型或外部投递目标。

use super::{CronJob, Schedule, SessionTarget, add_agent_job};
use crate::app::agent::config::Config;
use anyhow::Result;

/// Default cron expression: 3:00 AM daily.
const DEFAULT_SCHEDULE_EXPR: &str = "0 3 * * *";

/// 标识内置夜间归并任务的稳定名称。
pub const CONSOLIDATION_JOB_NAME: &str = "__consolidate_nightly";

/// The prompt instructs the agent to perform memory consolidation using
/// existing tools (cron_runs, memory_recall, memory_store, file_write).
const CONSOLIDATION_PROMPT: &str = "\
You are running a nightly memory consolidation job. Your goal is to distill \
the past 24 hours of operational activity into a concise, actionable summary \
stored in long-term memory.

Follow these steps exactly:

1. Use `cron_runs` to review recent job execution results from the past 24 hours. \
   Note any recurring errors, timeouts, or policy denials.

2. Use `memory_recall` to retrieve today's Daily memories. Look for patterns, \
   discoveries, and progress toward goals.

3. Identify and classify findings:
   - **Recurring errors**: problems that appeared more than once
   - **Successful strategies**: approaches that worked well
   - **New discoveries**: information or capabilities learned
   - **Blocked goals**: objectives that could not be completed and why

4. Synthesize a concise summary (max 500 words) of actionable learnings. \
   Focus on what should change going forward, not just what happened.

5. Store the summary using `memory_store` with category \"core\" and \
   key format \"consolidation_YYYY-MM-DD\" (use today's date).

6. If the workspace file `MEMORY.md` exists, use `file_read` to read it, \
   then use `file_write` to append a dated section at the end with the \
   top 3 learnings from today's consolidation. Format:
   ```
   ## Consolidation — YYYY-MM-DD
   1. <learning 1>
   2. <learning 2>
   3. <learning 3>
   ```

If there is no meaningful activity to consolidate (no runs, no daily memories), \
store a brief note confirming the check was performed and skip the MEMORY.md update.";

/// 按本地时间每天 03:00 创建夜间记忆归并代理任务。
///
/// Schedule: 3:00 AM daily (local time), configurable via `schedule_expr`.
/// Job type: agent with `__consolidate` marker in the name.
/// Session target: isolated (does not disturb main sessions).
pub fn create_consolidation_job(config: &Config) -> Result<CronJob> {
    create_consolidation_job_with_schedule(config, DEFAULT_SCHEDULE_EXPR, None)
}

/// 使用自定义 cron 表达式和可选时区创建隔离的归并任务。
pub fn create_consolidation_job_with_schedule(
    config: &Config,
    cron_expr: &str,
    tz: Option<String>,
) -> Result<CronJob> {
    let schedule = Schedule::Cron { expr: cron_expr.into(), tz };

    add_agent_job(
        config,
        Some(CONSOLIDATION_JOB_NAME.into()),
        schedule,
        CONSOLIDATION_PROMPT,
        SessionTarget::Isolated,
        None,  // use default model
        None,  // no delivery config
        false, // recurring job — do not delete after run
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
