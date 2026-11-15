//! Cron Store 模块测试
//!
//! 本模块包含针对 `src/app/agent/cron/store.rs` 中定义的定时任务存储功能的单元测试。
//!
//! # 测试范围
//!
//! - 定时任务的添加、查询、更新和删除（CRUD 操作）
//! - 任务调度时间的计算与过滤
//! - 任务执行历史记录的管理与清理
//! - 输出截断机制的正确性
//! - 数据库迁移和向后兼容性
//!
//! # 测试策略
//!
//! 每个测试使用独立的临时目录和临时数据库，确保测试之间相互隔离，
//! 不会产生副作用或依赖关系。

use agent_config::runtime::Config;
use crate::store::{
    MAX_CRON_OUTPUT_BYTES, TRUNCATED_OUTPUT_MARKER, add_job, add_shell_job, due_jobs, get_job,
    list_jobs, list_runs, record_run, remove_job, reschedule_after_run, update_job,
    with_connection,
};
use crate::{CronJobPatch, JobType, Schedule};
use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::params;
use tempfile::TempDir;

/// 创建用于测试的配置对象
///
/// # 参数
///
/// - `tmp`: 临时目录引用，用于隔离测试环境
///
/// # 返回值
///
/// 返回一个配置对象，其中：
/// - `workspace_dir` 设置为临时目录下的 `workspace` 子目录
/// - `config_path` 设置为临时目录下的 `vibewindow.json`
/// - 其他配置使用默认值
///
/// # 副作用
///
/// 会创建 `workspace_dir` 目录（如果不存在）
fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

/// 测试：添加任务时接受五字段 cron 表达式
///
/// 验证 `add_job` 函数能够正确解析和存储标准的五字段 cron 表达式。
///
/// # 测试场景
///
/// 1. 创建一个使用 `*/5 * * * *` 表达式的任务
/// 2. 验证任务的 expression 字段被正确存储
/// 3. 验证任务的 command 字段被正确存储
/// 4. 验证任务的 schedule 类型被识别为 Cron
#[test]
fn add_job_accepts_five_field_expression() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let job = add_job(&config, "*/5 * * * *", "echo ok").unwrap();
    assert_eq!(job.expression, "*/5 * * * *");
    assert_eq!(job.command, "echo ok");
    assert!(matches!(job.schedule, Schedule::Cron { .. }));
}

/// 测试：一次性任务的自动删除标记
///
/// 验证 `add_shell_job` 函数会根据调度类型自动设置 `delete_after_run` 标记：
/// - `Schedule::At`（一次性任务）应设置 `delete_after_run` 为 true
/// - `Schedule::Every`（周期性任务）应设置 `delete_after_run` 为 false
///
/// # 测试场景
///
/// 1. 创建一个 10 分钟后执行的一次性任务，验证其 `delete_after_run` 为 true
/// 2. 创建一个每分钟执行的周期性任务，验证其 `delete_after_run` 为 false
#[test]
fn add_shell_job_marks_at_schedule_for_auto_delete() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 创建一次性任务（Schedule::At），预期自动标记为执行后删除
    let one_shot = add_shell_job(
        &config,
        None,
        Schedule::At { at: Utc::now() + ChronoDuration::minutes(10) },
        "echo once",
    )
    .unwrap();
    assert!(one_shot.delete_after_run);

    // 创建周期性任务（Schedule::Every），预期不会自动删除
    let recurring =
        add_shell_job(&config, None, Schedule::Every { every_ms: 60_000 }, "echo recurring")
            .unwrap();
    assert!(!recurring.delete_after_run);
}

/// 测试：任务的添加、列表和删除完整流程
///
/// 验证任务存储的完整生命周期：添加 -> 列表查询 -> 删除
///
/// # 测试场景
///
/// 1. 添加一个新任务
/// 2. 通过 `list_jobs` 验证任务已存在于列表中
/// 3. 通过 `remove_job` 删除该任务
/// 4. 再次调用 `list_jobs` 验证任务已被删除
#[test]
fn add_list_remove_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 添加任务
    let job = add_job(&config, "*/10 * * * *", "echo roundtrip").unwrap();
    // 验证任务出现在列表中
    let listed = list_jobs(&config).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, job.id);

    // 删除任务并验证列表为空
    remove_job(&config, &job.id).unwrap();
    assert!(list_jobs(&config).unwrap().is_empty());
}

/// 测试：due_jobs 按时间戳和启用状态过滤任务
///
/// 验证 `due_jobs` 函数的过滤逻辑：
/// - 新创建的任务不应立即被标记为到期
/// - 当查询时间到达任务的下次运行时间时，任务应被返回
/// - 已禁用的任务不应被返回，即使时间已到
///
/// # 测试场景
///
/// 1. 创建任务后立即查询，验证无到期任务
/// 2. 查询一年后的时间点，验证任务被标记为到期
/// 3. 禁用任务后再次查询，验证已禁用任务被排除
#[test]
fn due_jobs_filters_by_timestamp_and_enabled() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let job = add_job(&config, "* * * * *", "echo due").unwrap();

    // 新创建的任务不应立即到期
    let due_now = due_jobs(&config, Utc::now()).unwrap();
    assert!(due_now.is_empty(), "new job should not be due immediately");

    // 查询一年后，任务应该到期
    let far_future = Utc::now() + ChronoDuration::days(365);
    let due_future = due_jobs(&config, far_future).unwrap();
    assert_eq!(due_future.len(), 1, "job should be due in far future");

    // 禁用任务后，即使时间已到也不应返回
    let _ = update_job(
        &config,
        &job.id,
        CronJobPatch { enabled: Some(false), ..CronJobPatch::default() },
    )
    .unwrap();
    let due_after_disable = due_jobs(&config, far_future).unwrap();
    assert!(due_after_disable.is_empty());
}

/// 测试：due_jobs 遵守调度器最大任务数限制
///
/// 验证 `due_jobs` 函数会遵守配置中的 `scheduler.max_tasks` 限制，
/// 即使有更多任务到期，也只返回允许的最大数量。
///
/// # 测试场景
///
/// 1. 将配置的 `max_tasks` 设置为 2
/// 2. 添加 3 个任务，理论上都应在远期到期
/// 3. 验证 `due_jobs` 只返回 2 个任务
#[test]
fn due_jobs_respects_scheduler_max_tasks_limit() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp);
    config.scheduler.max_tasks = 2;

    // 添加 3 个任务
    let _ = add_job(&config, "* * * * *", "echo due-1").unwrap();
    let _ = add_job(&config, "* * * * *", "echo due-2").unwrap();
    let _ = add_job(&config, "* * * * *", "echo due-3").unwrap();

    // 验证只返回 max_tasks 数量的任务
    let far_future = Utc::now() + ChronoDuration::days(365);
    let due = due_jobs(&config, far_future).unwrap();
    assert_eq!(due.len(), 2);
}

/// 测试：重新调度后持久化最后状态和运行时间
///
/// 验证 `reschedule_after_run` 函数在任务执行后会正确更新：
/// - `last_status`：根据执行结果设置为 "ok" 或 "error"
/// - `last_run`：记录最后执行时间
/// - `last_output`：记录最后的输出内容
///
/// # 测试场景
///
/// 1. 创建任务
/// 2. 调用 `reschedule_after_run` 模拟执行失败
/// 3. 验证任务状态被正确更新为 "error"
/// 4. 验证 `last_run` 和 `last_output` 被正确记录
#[test]
fn reschedule_after_run_persists_last_status_and_last_run() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let job = add_job(&config, "*/15 * * * *", "echo run").unwrap();
    // 模拟任务执行失败
    reschedule_after_run(&config, &job, false, "failed output").unwrap();

    let listed = list_jobs(&config).unwrap();
    let stored = listed.iter().find(|j| j.id == job.id).unwrap();
    // 验证状态被标记为 error
    assert_eq!(stored.last_status.as_deref(), Some("error"));
    // 验证最后运行时间已记录
    assert!(stored.last_run.is_some());
    // 验证输出被正确存储
    assert_eq!(stored.last_output.as_deref(), Some("failed output"));
}

/// 测试：从数据库读取有效的任务类型
///
/// 验证从 SQLite 数据库读取 `job_type` 字段时，能够正确解析有效的类型值。
///
/// # 测试场景
///
/// 1. 直接向数据库插入一条 `job_type` 为 "agent" 的任务记录
/// 2. 通过 `get_job` 读取该任务
/// 3. 验证 `job_type` 被正确解析为 `JobType::Agent` 枚举值
#[test]
fn job_type_from_sql_reads_valid_value() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let now = Utc::now();

    // 直接插入带有 job_type 字段的记录
    with_connection(&config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, schedule, job_type, created_at, next_run)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "job-type-valid",
                "*/5 * * * *",
                "echo ok",
                Option::<String>::None,
                "agent",
                now.to_rfc3339(),
                (now + ChronoDuration::minutes(5)).to_rfc3339(),
            ],
        )?;
        Ok(())
    })
    .unwrap();

    let job = get_job(&config, "job-type-valid").unwrap();
    assert_eq!(job.job_type, JobType::Agent);
}

/// 测试：从数据库读取时拒绝无效的任务类型
///
/// 验证从 SQLite 数据库读取 `job_type` 字段时，遇到无效值会返回错误，
/// 而不是静默地使用默认值或产生未定义行为。
///
/// # 测试场景
///
/// 1. 直接向数据库插入一条 `job_type` 为 "unknown"（无效值）的任务记录
/// 2. 尝试通过 `get_job` 读取该任务
/// 3. 验证操作返回错误
#[test]
fn job_type_from_sql_rejects_invalid_value() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let now = Utc::now();

    // 插入带有无效 job_type 的记录
    with_connection(&config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, schedule, job_type, created_at, next_run)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "job-type-invalid",
                "*/5 * * * *",
                "echo ok",
                Option::<String>::None,
                "unknown",
                now.to_rfc3339(),
                (now + ChronoDuration::minutes(5)).to_rfc3339(),
            ],
        )?;
        Ok(())
    })
    .unwrap();

    // 验证读取无效 job_type 会返回错误
    assert!(get_job(&config, "job-type-invalid").is_err());
}

/// 测试：迁移时回退到旧版 expression 字段
///
/// 验证数据库迁移的向后兼容性：当 `schedule` 字段为 NULL 时，
/// 系统应能够回退到使用旧版的 `expression` 字段构建 `Schedule::Cron`。
///
/// # 测试场景
///
/// 1. 直接向数据库插入一条 `schedule` 字段为 NULL 的旧版记录
/// 2. 通过 `get_job` 读取该任务
/// 3. 验证系统使用 `expression` 字段正确构建了 `Schedule::Cron`
#[test]
fn migration_falls_back_to_legacy_expression() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 插入旧版记录，schedule 字段为 NULL
    with_connection(&config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, created_at, next_run)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "legacy-id",
                "*/5 * * * *",
                "echo legacy",
                Utc::now().to_rfc3339(),
                (Utc::now() + ChronoDuration::minutes(5)).to_rfc3339(),
            ],
        )?;
        // 显式将 schedule 设置为 NULL，模拟旧版数据
        conn.execute("UPDATE cron_jobs SET schedule = NULL WHERE id = 'legacy-id'", [])?;
        Ok(())
    })
    .unwrap();

    // 验证读取时能够正确回退到 expression 字段
    let job = get_job(&config, "legacy-id").unwrap();
    assert!(matches!(job.schedule, Schedule::Cron { .. }));
}

/// 测试：记录运行历史并自动清理旧记录
///
/// 验证 `record_run` 函数能够正确记录执行历史，并且当历史记录数量
/// 超过配置的 `max_run_history` 限制时，会自动清理最旧的记录。
///
/// # 测试场景
///
/// 1. 将 `max_run_history` 配置为 2
/// 2. 创建任务并记录 3 次执行
/// 3. 验证只保留了最新的 2 条记录
#[test]
fn record_and_prune_runs() {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(&tmp);
    // 限制最多保留 2 条历史记录
    config.cron.max_run_history = 2;
    let job = add_job(&config, "*/5 * * * *", "echo ok").unwrap();
    let base = Utc::now();

    // 记录 3 次执行，每次间隔 1 秒
    for idx in 0..3 {
        let start = base + ChronoDuration::seconds(idx);
        let end = start + ChronoDuration::milliseconds(100);
        record_run(&config, &job.id, start, end, "ok", Some("done"), 100).unwrap();
    }

    // 验证只保留了最新的 2 条记录
    let runs = list_runs(&config, &job.id, 10).unwrap();
    assert_eq!(runs.len(), 2);
}

/// 测试：删除任务时级联删除运行历史
///
/// 验证数据库的外键级联删除配置正确：当删除一个任务时，
/// 其所有关联的运行历史记录也应被自动删除。
///
/// # 测试场景
///
/// 1. 创建任务并记录一次执行历史
/// 2. 验证运行历史存在
/// 3. 删除任务
/// 4. 验证运行历史已被级联删除
#[test]
fn remove_job_cascades_run_history() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "*/5 * * * *", "echo ok").unwrap();
    let start = Utc::now();
    // 记录一次执行
    record_run(
        &config,
        &job.id,
        start,
        start + ChronoDuration::milliseconds(5),
        "ok",
        Some("ok"),
        5,
    )
    .unwrap();

    // 删除任务
    remove_job(&config, &job.id).unwrap();
    // 验证运行历史已被级联删除
    let runs = list_runs(&config, &job.id, 10).unwrap();
    assert!(runs.is_empty());
}

/// 测试：记录运行时截断过大的输出
///
/// 验证 `record_run` 函数在存储输出时会检查输出大小，
/// 当输出超过 `MAX_CRON_OUTPUT_BYTES` 限制时，会自动截断并添加截断标记。
///
/// # 测试场景
///
/// 1. 创建一个超过 `MAX_CRON_OUTPUT_BYTES` 限制的大字符串作为输出
/// 2. 调用 `record_run` 记录该输出
/// 3. 验证存储的输出被正确截断
/// 4. 验证截断后的输出以 `TRUNCATED_OUTPUT_MARKER` 结尾
/// 5. 验证截断后的输出不超过 `MAX_CRON_OUTPUT_BYTES`
#[test]
fn record_run_truncates_large_output() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "*/5 * * * *", "echo trunc").unwrap();
    // 创建一个超过限制的输出字符串
    let output = "x".repeat(MAX_CRON_OUTPUT_BYTES + 512);

    // 记录带有大输出的执行
    record_run(&config, &job.id, Utc::now(), Utc::now(), "ok", Some(&output), 1).unwrap();

    // 验证输出被正确截断
    let runs = list_runs(&config, &job.id, 1).unwrap();
    let stored = runs[0].output.as_deref().unwrap_or_default();
    // 验证输出以截断标记结尾
    assert!(stored.ends_with(TRUNCATED_OUTPUT_MARKER));
    // 验证输出大小在限制范围内
    assert!(stored.len() <= MAX_CRON_OUTPUT_BYTES);
}

/// 测试：重新调度时截断过大的最后输出
///
/// 验证 `reschedule_after_run` 函数在更新任务的 `last_output` 字段时，
/// 也会检查输出大小并进行截断处理，与 `record_run` 保持一致的行为。
///
/// # 测试场景
///
/// 1. 创建任务
/// 2. 创建一个超过 `MAX_CRON_OUTPUT_BYTES` 限制的大字符串作为输出
/// 3. 调用 `reschedule_after_run` 更新任务状态
/// 4. 验证任务的 `last_output` 被正确截断
/// 5. 验证截断后的输出以 `TRUNCATED_OUTPUT_MARKER` 结尾
/// 6. 验证截断后的输出不超过 `MAX_CRON_OUTPUT_BYTES`
#[test]
fn reschedule_after_run_truncates_last_output() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "*/5 * * * *", "echo trunc").unwrap();
    // 创建一个超过限制的输出字符串
    let output = "y".repeat(MAX_CRON_OUTPUT_BYTES + 1024);

    // 重新调度时传入大输出
    reschedule_after_run(&config, &job, false, &output).unwrap();

    // 验证 last_output 被正确截断
    let stored = get_job(&config, &job.id).unwrap();
    let last_output = stored.last_output.as_deref().unwrap_or_default();
    // 验证输出以截断标记结尾
    assert!(last_output.ends_with(TRUNCATED_OUTPUT_MARKER));
    // 验证输出大小在限制范围内
    assert!(last_output.len() <= MAX_CRON_OUTPUT_BYTES);
}
