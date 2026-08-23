//! 定时任务领域入口，汇集计划表达、持久化、调度器与命令行操作。

use crate::app::agent::config::Config;
use crate::app::agent::security::SecurityPolicy;
use anyhow::{Result, bail};

pub mod consolidation;
mod schedule;
mod store;
mod types;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;

pub mod scheduler;

#[allow(unused_imports)]
pub use schedule::{
    next_run_for_schedule, normalize_expression, schedule_cron_expression, validate_schedule,
};
#[allow(unused_imports)]
pub use store::{
    add_agent_job, add_job, add_shell_job, due_jobs, get_job, list_jobs, list_runs,
    record_last_run, record_run, remove_job, reschedule_after_run, update_job,
};
pub(crate) use types::normalize_fallbacks;
pub use types::{CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget};

use clap::Subcommand;

#[derive(Debug, Clone, Subcommand)]
pub enum CronCommands {
    /// List all scheduled jobs
    List,
    /// Add a new cron job with a cron expression
    Add {
        /// Cron expression (e.g. '0 9 * * 1-5')
        expression: String,
        /// Timezone (e.g. America/New_York)
        #[arg(long)]
        tz: Option<String>,
        /// Command to execute
        command: String,
    },
    /// Add a one-shot job at a specific time (RFC 3339)
    AddAt {
        /// RFC 3339 timestamp (e.g. 2025-01-15T14:00:00Z)
        at: String,
        /// Command to execute
        command: String,
    },
    /// Add a job that runs every N milliseconds
    AddEvery {
        /// Interval in milliseconds
        every_ms: u64,
        /// Command to execute
        command: String,
    },
    /// Run a command once after a delay
    Once {
        /// Delay (e.g. 30m, 1h)
        delay: String,
        /// Command to execute
        command: String,
    },
    /// Remove a job by ID
    Remove { id: String },
    /// Update an existing job
    Update {
        id: String,
        #[arg(long)]
        expression: Option<String>,
        #[arg(long)]
        tz: Option<String>,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// Pause a job by ID
    Pause { id: String },
    /// Resume a job by ID
    Resume { id: String },
}

#[allow(clippy::needless_pass_by_value)]
pub fn handle_command(command: CronCommands, config: &Config) -> Result<()> {
    match command {
        CronCommands::List => {
            let jobs = list_jobs(config)?;
            if jobs.is_empty() {
                println!("No scheduled tasks yet.");
                println!("\nUsage:");
                println!("  vibewindow cron add '0 9 * * *' 'agent -m \"Good morning!\"'");
                return Ok(());
            }

            println!("🕒 Scheduled jobs ({}):", jobs.len());
            for job in jobs {
                let last_run = job.last_run.map_or_else(|| "never".into(), |d| d.to_rfc3339());
                let last_status = job.last_status.unwrap_or_else(|| "n/a".into());
                println!(
                    "- {} | {:?} | next={} | last={} ({})",
                    job.id,
                    job.schedule,
                    job.next_run.to_rfc3339(),
                    last_run,
                    last_status,
                );
                if !job.command.is_empty() {
                    println!("    cmd: {}", job.command);
                }
                if let Some(prompt) = &job.prompt {
                    println!("    prompt: {prompt}");
                }
            }
            Ok(())
        }
        CronCommands::Add { expression, tz, command } => {
            let schedule = Schedule::Cron { expr: expression, tz };
            let job = add_shell_job(config, None, schedule, &command)?;
            println!("✅ Added cron job {}", job.id);
            println!("  Expr: {}", job.expression);
            println!("  Next: {}", job.next_run.to_rfc3339());
            println!("  Cmd : {}", job.command);
            Ok(())
        }
        CronCommands::AddAt { at, command } => {
            let at = chrono::DateTime::parse_from_rfc3339(&at)
                .map_err(|e| anyhow::anyhow!("Invalid RFC3339 timestamp for --at: {e}"))?
                .with_timezone(&chrono::Utc);
            let schedule = Schedule::At { at };
            let job = add_shell_job(config, None, schedule, &command)?;
            println!("✅ Added one-shot cron job {}", job.id);
            println!("  At  : {}", job.next_run.to_rfc3339());
            println!("  Cmd : {}", job.command);
            Ok(())
        }
        CronCommands::AddEvery { every_ms, command } => {
            let schedule = Schedule::Every { every_ms };
            let job = add_shell_job(config, None, schedule, &command)?;
            println!("✅ Added interval cron job {}", job.id);
            println!("  Every(ms): {every_ms}");
            println!("  Next     : {}", job.next_run.to_rfc3339());
            println!("  Cmd      : {}", job.command);
            Ok(())
        }
        CronCommands::Once { delay, command } => {
            let job = add_once(config, &delay, &command)?;
            println!("✅ Added one-shot cron job {}", job.id);
            println!("  At  : {}", job.next_run.to_rfc3339());
            println!("  Cmd : {}", job.command);
            Ok(())
        }
        CronCommands::Update { id, expression, tz, command, name } => {
            if expression.is_none() && tz.is_none() && command.is_none() && name.is_none() {
                bail!("At least one of --expression, --tz, --command, or --name must be provided");
            }

            // Merge expression/tz with the existing schedule so that
            // --tz alone updates the timezone and --expression alone
            // preserves the existing timezone.
            let schedule = if expression.is_some() || tz.is_some() {
                let existing = get_job(config, &id)?;
                let (existing_expr, existing_tz) = match existing.schedule {
                    Schedule::Cron { expr, tz: existing_tz } => (expr, existing_tz),
                    _ => bail!("Cannot update expression/tz on a non-cron schedule"),
                };
                Some(Schedule::Cron {
                    expr: expression.unwrap_or(existing_expr),
                    tz: tz.or(existing_tz),
                })
            } else {
                None
            };

            if let Some(ref cmd) = command {
                let security = SecurityPolicy::from_configs(
                    &config.autonomy,
                    &config.security,
                    &config.workspace_dir,
                );
                if !security.is_command_allowed(cmd) {
                    bail!("Command blocked by security policy: {cmd}");
                }
            }

            let patch = CronJobPatch { schedule, command, name, ..CronJobPatch::default() };

            let job = update_job(config, &id, patch)?;
            println!("\u{2705} Updated cron job {}", job.id);
            println!("  Expr: {}", job.expression);
            println!("  Next: {}", job.next_run.to_rfc3339());
            println!("  Cmd : {}", job.command);
            Ok(())
        }
        CronCommands::Remove { id } => remove_job(config, &id),
        CronCommands::Pause { id } => {
            pause_job(config, &id)?;
            println!("⏸️  Paused cron job {id}");
            Ok(())
        }
        CronCommands::Resume { id } => {
            resume_job(config, &id)?;
            println!("▶️  Resumed cron job {id}");
            Ok(())
        }
    }
}

pub fn add_once(config: &Config, delay: &str, command: &str) -> Result<CronJob> {
    let duration = parse_delay(delay)?;
    let at = chrono::Utc::now() + duration;
    add_once_at(config, at, command)
}

pub fn add_once_at(
    config: &Config,
    at: chrono::DateTime<chrono::Utc>,
    command: &str,
) -> Result<CronJob> {
    let schedule = Schedule::At { at };
    add_shell_job(config, None, schedule, command)
}

pub fn pause_job(config: &Config, id: &str) -> Result<CronJob> {
    update_job(config, id, CronJobPatch { enabled: Some(false), ..CronJobPatch::default() })
}

pub fn resume_job(config: &Config, id: &str) -> Result<CronJob> {
    update_job(config, id, CronJobPatch { enabled: Some(true), ..CronJobPatch::default() })
}

fn parse_delay(input: &str) -> Result<chrono::Duration> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("delay must not be empty");
    }
    let split = input.find(|c: char| !c.is_ascii_digit()).unwrap_or(input.len());
    let (num, unit) = input.split_at(split);
    let amount: i64 = num.parse()?;
    let unit = if unit.is_empty() { "m" } else { unit };
    let duration = match unit {
        "s" => chrono::Duration::seconds(amount),
        "m" => chrono::Duration::minutes(amount),
        "h" => chrono::Duration::hours(amount),
        "d" => chrono::Duration::days(amount),
        _ => anyhow::bail!("unsupported delay unit '{unit}', use s/m/h/d"),
    };
    Ok(duration)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
