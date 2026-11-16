//! Real host execution and tenant context injected into the domain history matrix.
use super::execute_job_now;
use tool_cron::history_test_support as history;
#[tokio::test]
async fn force_runs_job_and_records_history() { history::force_runs_job_and_records_history(execute_job_now).await; }
#[tokio::test]
async fn shell_run_requires_approval_for_medium_risk() { history::shell_run_requires_approval_for_medium_risk(execute_job_now).await; }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_history_manual_run_is_visible_until_completion() { history::cron_history_manual_run_is_visible_until_completion(execute_job_now).await; }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_history_cancelled_manual_run_leaves_no_history() { history::cron_history_cancelled_manual_run_leaves_no_history(execute_job_now).await; }
