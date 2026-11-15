pub use tool_cron::cron_run::*;

use crate::app::agent::{config::Config, cron};

/// 将现有宿主执行动作直接交给领域工具，保留取消语义。
pub(crate) fn execute_job_now<'a>(config: &'a Config, job: &'a cron::CronJob) -> tool_cron::cron_run::JobExecutionFuture<'a> {
    Box::pin(cron::scheduler::execute_job_now(config, job))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
