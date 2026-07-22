//! Cron 调度模块测试套件
//!
//! 本模块提供了对 cron 任务调度功能的全面测试覆盖，包括：
//! - 任务创建和配置
//! - 任务更新操作（命令、表达式、时区、名称）
//! - 字段保持和变更验证
//! - 安全策略验证
//! - 错误处理场景
//!
//! 所有测试使用临时目录隔离，确保测试之间的独立性和可重复性。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    /// 创建用于测试的配置对象
    ///
    /// 生成一个带有临时工作空间目录和配置路径的测试配置，
    /// 并创建必要的工作空间目录结构。
    ///
    /// # 参数
    ///
    /// * `tmp` - 临时目录引用，用于隔离测试环境
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `Config` 实例，包含：
    /// - `workspace_dir`: 临时目录下的工作空间路径
    /// - `config_path`: 临时目录下的配置文件路径
    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("vibewindow.json"),
            ..Config::default()
        };
        // 创建工作空间目录，确保测试环境完整
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    /// 创建用于测试的 cron 任务
    ///
    /// 使用指定的 cron 表达式和命令创建一个新的 shell 任务，
    /// 用于后续的测试操作。
    ///
    /// # 参数
    ///
    /// * `config` - 配置对象引用
    /// * `expr` - cron 表达式字符串（如 "*/5 * * * *"）
    /// * `tz` - 可选的时区字符串（如 "America/Los_Angeles"）
    /// * `cmd` - 要执行的 shell 命令
    ///
    /// # 返回值
    ///
    /// 返回创建成功的 `CronJob` 实例
    fn make_job(config: &Config, expr: &str, tz: Option<&str>, cmd: &str) -> CronJob {
        add_shell_job(
            config,
            None,
            Schedule::Cron { expr: expr.into(), tz: tz.map(Into::into) },
            cmd,
        )
        .unwrap()
    }

    /// 执行任务更新操作的辅助函数
    ///
    /// 封装了 `handle_command` 调用，简化测试中的更新操作。
    /// 允许更新任务的一个或多个字段。
    ///
    /// # 参数
    ///
    /// * `config` - 配置对象引用
    /// * `id` - 要更新的任务 ID
    /// * `expression` - 可选的新 cron 表达式
    /// * `tz` - 可选的新时区
    /// * `command` - 可选的新命令
    /// * `name` - 可选的新任务名称
    ///
    /// # 返回值
    ///
    /// 返回更新操作的结果，成功为 `Ok(())`，失败为 `Err`
    fn run_update(
        config: &Config,
        id: &str,
        expression: Option<&str>,
        tz: Option<&str>,
        command: Option<&str>,
        name: Option<&str>,
    ) -> Result<()> {
        handle_command(
            CronCommands::Update {
                id: id.into(),
                expression: expression.map(Into::into),
                tz: tz.map(Into::into),
                command: command.map(Into::into),
                name: name.map(Into::into),
            },
            config,
        )
    }

    /// 测试通过处理器更新任务命令
    ///
    /// 验证：
    /// - 命令字段能够被正确更新
    /// - 其他字段（如 ID）保持不变
    /// - 更新后的任务可以成功检索
    #[test]
    fn update_changes_command_via_handler() {
        // 创建临时测试环境
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        // 创建初始任务
        let job = make_job(&config, "*/5 * * * *", None, "echo original");

        // 执行命令更新
        run_update(&config, &job.id, None, None, Some("echo updated"), None).unwrap();

        // 验证更新结果：命令已变更，ID 保持不变
        let updated = get_job(&config, &job.id).unwrap();
        assert_eq!(updated.command, "echo updated");
        assert_eq!(updated.id, job.id);
    }

    /// 测试通过处理器更新任务表达式
    ///
    /// 验证：
    /// - cron 表达式能够被正确更新
    /// - 表达式格式被正确存储
    #[test]
    fn update_changes_expression_via_handler() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let job = make_job(&config, "*/5 * * * *", None, "echo test");

        // 更新 cron 表达式从 "*/5 * * * *" 到 "0 9 * * *"
        run_update(&config, &job.id, Some("0 9 * * *"), None, None, None).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        assert_eq!(updated.expression, "0 9 * * *");
    }

    /// 测试通过处理器更新任务名称
    ///
    /// 验证任务名称字段能够被正确更新
    #[test]
    fn update_changes_name_via_handler() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let job = make_job(&config, "*/5 * * * *", None, "echo test");

        // 更新任务名称
        run_update(&config, &job.id, None, None, None, Some("new-name")).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        assert_eq!(updated.name.as_deref(), Some("new-name"));
    }

    /// 测试单独更新时区字段
    ///
    /// 验证：
    /// - 时区可以独立于表达式单独更新
    /// - 时区信息被正确存储在 Schedule 枚举中
    #[test]
    fn update_tz_alone_sets_timezone() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        // 创建没有时区的任务
        let job = make_job(&config, "*/5 * * * *", None, "echo test");

        // 单独设置时区
        run_update(&config, &job.id, None, Some("America/Los_Angeles"), None, None).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        // 验证 schedule 包含正确的表达式和时区
        assert_eq!(
            updated.schedule,
            Schedule::Cron { expr: "*/5 * * * *".into(), tz: Some("America/Los_Angeles".into()) }
        );
    }

    /// 测试更新表达式时保留现有时区
    ///
    /// 验证：
    /// - 更新表达式字段时，已存在的时区信息不被清除
    /// - 多个字段可以独立管理，不会相互干扰
    #[test]
    fn update_expression_preserves_existing_tz() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        // 创建带有时区的任务
        let job = make_job(&config, "*/5 * * * *", Some("America/Los_Angeles"), "echo test");

        // 只更新表达式，不修改时区
        run_update(&config, &job.id, Some("0 9 * * *"), None, None, None).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        // 验证时区被保留，表达式已更新
        assert_eq!(
            updated.schedule,
            Schedule::Cron { expr: "0 9 * * *".into(), tz: Some("America/Los_Angeles".into()) }
        );
    }

    /// 测试更新操作保留未变更的字段
    ///
    /// 验证：
    /// - 只更新指定字段，其他字段保持原值
    /// - 部分更新不会导致数据丢失
    /// - 名称、表达式等字段在命令更新时保持不变
    #[test]
    fn update_preserves_unchanged_fields() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        // 创建带有名称的任务
        let job = add_shell_job(
            &config,
            Some("original-name".into()),
            Schedule::Cron { expr: "*/5 * * * *".into(), tz: None },
            "echo original",
        )
        .unwrap();

        // 只更新命令字段
        run_update(&config, &job.id, None, None, Some("echo changed"), None).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        // 验证命令已变更
        assert_eq!(updated.command, "echo changed");
        // 验证名称保持不变
        assert_eq!(updated.name.as_deref(), Some("original-name"));
        // 验证表达式保持不变
        assert_eq!(updated.expression, "*/5 * * * *");
    }

    /// 测试无任何更新标志时更新失败
    ///
    /// 验证：
    /// - 必须至少指定一个要更新的字段
    /// - 错误消息包含有用的提示信息
    #[test]
    fn update_no_flags_fails() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let job = make_job(&config, "*/5 * * * *", None, "echo test");

        // 尝试不指定任何更新字段的操作
        let result = run_update(&config, &job.id, None, None, None, None);
        assert!(result.is_err());
        // 验证错误消息包含提示信息
        assert!(result.unwrap_err().to_string().contains("At least one of"));
    }

    /// 测试更新不存在的任务时失败
    ///
    /// 验证尝试更新不存在的任务 ID 时会返回错误
    #[test]
    fn update_nonexistent_job_fails() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        // 尝试更新不存在的任务
        let result = run_update(&config, "nonexistent-id", None, None, Some("echo test"), None);
        assert!(result.is_err());
    }

    /// 测试安全策略允许安全命令
    ///
    /// 验证：
    /// - 安全策略能够正确初始化
    /// - 基本的安全命令（如 echo）被允许执行
    #[test]
    fn update_security_allows_safe_command() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        // 从配置创建安全策略
        let security = SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir);
        // 验证安全命令被允许
        assert!(security.is_command_allowed("echo safe"));
    }

    #[test]
    fn parse_delay_accepts_supported_units_and_default_minutes() {
        assert_eq!(parse_delay("15").unwrap(), chrono::Duration::minutes(15));
        assert_eq!(parse_delay(" 2s ").unwrap(), chrono::Duration::seconds(2));
        assert_eq!(parse_delay("3m").unwrap(), chrono::Duration::minutes(3));
        assert_eq!(parse_delay("4h").unwrap(), chrono::Duration::hours(4));
        assert_eq!(parse_delay("5d").unwrap(), chrono::Duration::days(5));
    }

    #[test]
    fn parse_delay_rejects_empty_values_and_unknown_units() {
        let empty = parse_delay("").unwrap_err().to_string();
        assert!(empty.contains("delay must not be empty"));

        let unit = parse_delay("10w").unwrap_err().to_string();
        assert!(unit.contains("unsupported delay unit"));
    }

    #[test]
    fn parse_delay_rejects_missing_and_out_of_range_amounts() {
        let missing = parse_delay("hours").unwrap_err().to_string();
        assert!(missing.contains("cannot parse integer"));

        let out_of_range = parse_delay("9223372036854775808s").unwrap_err().to_string();
        assert!(out_of_range.contains("number too large"));
    }

    #[test]
    fn add_once_at_and_pause_resume_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let at = chrono::Utc::now() + chrono::Duration::minutes(30);

        let job = add_once_at(&config, at, "echo once").unwrap();
        assert!(job.delete_after_run);
        assert_eq!(job.command, "echo once");
        assert_eq!(job.schedule, Schedule::At { at });

        let paused = pause_job(&config, &job.id).unwrap();
        assert!(!paused.enabled);

        let resumed = resume_job(&config, &job.id).unwrap();
        assert!(resumed.enabled);
    }

    #[test]
    fn handle_command_add_variants_list_and_remove_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

        handle_command(
            CronCommands::Add {
                expression: "*/10 * * * *".into(),
                tz: Some("UTC".into()),
                command: "echo cron".into(),
            },
            &config,
        )
        .unwrap();
        handle_command(CronCommands::AddAt { at, command: "echo at".into() }, &config).unwrap();
        handle_command(
            CronCommands::AddEvery { every_ms: 60_000, command: "echo every".into() },
            &config,
        )
        .unwrap();
        handle_command(
            CronCommands::Once { delay: "1h".into(), command: "echo once".into() },
            &config,
        )
        .unwrap();
        handle_command(CronCommands::List, &config).unwrap();

        let jobs = list_jobs(&config).unwrap();
        assert_eq!(jobs.len(), 4);
        assert!(jobs.iter().any(|job| job.command == "echo cron"));
        assert!(jobs.iter().any(|job| job.command == "echo at"));
        assert!(jobs.iter().any(|job| job.command == "echo every"));
        assert!(jobs.iter().any(|job| job.command == "echo once"));

        handle_command(CronCommands::Remove { id: jobs[0].id.clone() }, &config).unwrap();
        assert_eq!(list_jobs(&config).unwrap().len(), 3);
    }

    #[test]
    fn handle_command_list_empty_is_ok() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        handle_command(CronCommands::List, &config).unwrap();
    }

    #[test]
    fn add_at_normalizes_offset_timestamp_to_utc() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let at = "2099-01-15T14:00:00+08:00";
        let expected =
            chrono::DateTime::parse_from_rfc3339(at).unwrap().with_timezone(&chrono::Utc);

        handle_command(
            CronCommands::AddAt { at: at.into(), command: "echo offset".into() },
            &config,
        )
        .unwrap();

        let jobs = list_jobs(&config).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].schedule, Schedule::At { at: expected });
        assert_eq!(jobs[0].command, "echo offset");
    }

    #[test]
    fn update_timezone_replaces_existing_timezone() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let job = make_job(&config, "*/5 * * * *", Some("UTC"), "echo test");

        run_update(&config, &job.id, None, Some("Asia/Shanghai"), None, None).unwrap();

        let updated = get_job(&config, &job.id).unwrap();
        assert_eq!(
            updated.schedule,
            Schedule::Cron { expr: "*/5 * * * *".into(), tz: Some("Asia/Shanghai".into()) }
        );
    }

    #[test]
    fn update_expression_for_missing_job_returns_lookup_error() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let error = run_update(&config, "missing-job", Some("0 9 * * *"), None, None, None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("Cron job 'missing-job' not found"));
    }

    #[test]
    fn blocked_command_does_not_apply_requested_schedule_change() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.autonomy.allowed_commands = vec!["echo".into()];
        let job = make_job(&config, "*/5 * * * *", Some("UTC"), "echo original");

        let error = run_update(
            &config,
            &job.id,
            Some("0 9 * * *"),
            Some("Asia/Shanghai"),
            Some("date"),
            None,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Command blocked by security policy"));
        let unchanged = get_job(&config, &job.id).unwrap();
        assert_eq!(
            unchanged.schedule,
            Schedule::Cron { expr: "*/5 * * * *".into(), tz: Some("UTC".into()) }
        );
        assert_eq!(unchanged.command, "echo original");
    }

    #[test]
    fn missing_job_mutations_propagate_not_found_errors() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        for command in [
            CronCommands::Pause { id: "missing-pause".into() },
            CronCommands::Resume { id: "missing-resume".into() },
            CronCommands::Remove { id: "missing-remove".into() },
        ] {
            let error = handle_command(command, &config).unwrap_err().to_string();
            assert!(error.contains("not found"));
        }
    }

    #[test]
    fn update_expression_or_tz_rejects_non_cron_schedule() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        let at = chrono::Utc::now() + chrono::Duration::minutes(30);
        let job = add_once_at(&config, at, "echo once").unwrap();

        let expression_err = run_update(&config, &job.id, Some("0 9 * * *"), None, None, None)
            .unwrap_err()
            .to_string();
        assert!(expression_err.contains("Cannot update expression/tz on a non-cron schedule"));

        let tz_err =
            run_update(&config, &job.id, None, Some("UTC"), None, None).unwrap_err().to_string();
        assert!(tz_err.contains("Cannot update expression/tz on a non-cron schedule"));
    }
}
