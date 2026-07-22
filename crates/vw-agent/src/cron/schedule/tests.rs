//! 定时任务调度模块的单元测试
//!
//! 本模块包含对 cron 表达式解析、时区处理以及下次运行时间计算等功能的测试用例。
//! 主要验证：
//! - 不同时区下的 cron 表达式解析
//! - 下次运行时间的正确计算
//! - 时区转换的准确性

use super::*;

/// 定时任务调度测试模块
///
/// 包含对 Schedule 相关功能的单元测试，验证调度器在各种场景下的行为。
#[allow(dead_code)]
mod tests {
    use super::*;

    use chrono::TimeZone;

    /// 测试 `next_run_for_schedule` 函数对时区的支持
    ///
    /// 验证点：
    /// - cron 表达式能够正确解析指定的时区
    /// - 下次运行时间能够正确转换为目标时区的时间
    /// - UTC 时间与指定时区时间的转换准确无误
    ///
    /// 测试场景：
    /// - 从 UTC 时间 `2026-02-16 00:00:00` 开始
    /// - cron 表达式 `0 9 * * *`（每天上午 9 点执行）
    /// - 时区设置为 `America/Los_Angeles`（洛杉矶时区，UTC-8）
    /// - 期望下次运行时间为 UTC `2026-02-16 17:00:00`
    ///
    /// 时区转换说明：
    /// 洛杉矶时间 09:00 对应 UTC 时间 17:00（洛杉矶为 UTC-8）
    #[test]
    fn next_run_for_schedule_supports_timezone() {
        // 设置起始时间点：2026年2月16日 00:00:00 UTC
        let from = Utc.with_ymd_and_hms(2026, 2, 16, 0, 0, 0).unwrap();

        // 创建定时计划：每天洛杉矶时间 09:00 执行
        let schedule =
            Schedule::Cron { expr: "0 9 * * *".into(), tz: Some("America/Los_Angeles".into()) };

        // 计算下次运行时间
        let next = next_run_for_schedule(&schedule, from).unwrap();

        // 验证：洛杉矶时间 09:00 = UTC 17:00
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 2, 16, 17, 0, 0).unwrap());
    }

    #[test]
    fn normalize_expression_handles_supported_field_counts() {
        assert_eq!(normalize_expression(" 0 9 * * * ").unwrap(), "0 0 9 * * *");
        assert_eq!(normalize_expression("5 0 9 * * *").unwrap(), "5 0 9 * * *");
        assert_eq!(normalize_expression("5 0 9 * * * 2026").unwrap(), "5 0 9 * * * 2026");
    }

    #[test]
    fn normalize_expression_rejects_unsupported_field_counts() {
        let err = normalize_expression("* * * *").unwrap_err().to_string();
        assert!(err.contains("expected 5, 6, or 7 fields"));

        let empty = normalize_expression("   ").unwrap_err().to_string();
        assert!(empty.contains("got 0"));
    }

    #[test]
    fn next_run_for_at_and_every_schedules() {
        let from = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let at = Utc.with_ymd_and_hms(2026, 6, 12, 11, 30, 0).unwrap();

        assert_eq!(next_run_for_schedule(&Schedule::At { at }, from).unwrap(), at);
        assert_eq!(
            next_run_for_schedule(&Schedule::Every { every_ms: 90_000 }, from).unwrap(),
            Utc.with_ymd_and_hms(2026, 6, 12, 10, 1, 30).unwrap()
        );
    }

    #[test]
    fn next_run_for_cron_without_timezone_uses_utc() {
        let from = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "30 10 * * *".into(), tz: None };

        let next = next_run_for_schedule(&schedule, from).unwrap();

        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 12, 10, 30, 0).unwrap());
    }

    #[test]
    fn next_run_for_every_rejects_zero_and_oversized_interval() {
        let from = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();

        let zero =
            next_run_for_schedule(&Schedule::Every { every_ms: 0 }, from).unwrap_err().to_string();
        assert!(zero.contains("every_ms must be > 0"));

        let oversized = next_run_for_schedule(&Schedule::Every { every_ms: u64::MAX }, from)
            .unwrap_err()
            .to_string();
        assert!(oversized.contains("every_ms is too large"));
    }

    #[test]
    fn next_run_for_every_rejects_datetime_overflow() {
        let from = Utc.with_ymd_and_hms(262142, 12, 31, 23, 59, 59).unwrap();

        let err = next_run_for_schedule(&Schedule::Every { every_ms: 1_000 }, from)
            .unwrap_err()
            .to_string();

        assert!(err.contains("every_ms overflowed DateTime"));
    }

    #[test]
    fn next_run_for_cron_rejects_invalid_expression_syntax() {
        let from = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "60 10 * * *".into(), tz: None };

        let err = next_run_for_schedule(&schedule, from).unwrap_err().to_string();

        assert!(err.contains("Invalid cron expression"));
    }

    #[test]
    fn next_run_for_cron_rejects_unsupported_field_count() {
        let from = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "0 10 * *".into(), tz: None };

        let err = next_run_for_schedule(&schedule, from).unwrap_err().to_string();

        assert!(err.contains("expected 5, 6, or 7 fields"));
        assert!(err.contains("got 4"));
    }

    #[test]
    fn next_run_for_cron_without_timezone_rejects_no_future_occurrence() {
        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "0 0 0 1 1 * 2020".into(), tz: None };

        let err = next_run_for_schedule(&schedule, from).unwrap_err().to_string();

        assert!(err.contains("No future occurrence"));
        assert!(err.contains("0 0 0 1 1 * 2020"));
    }

    #[test]
    fn next_run_for_cron_with_timezone_rejects_no_future_occurrence() {
        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let schedule =
            Schedule::Cron { expr: "0 0 0 1 1 * 2020".into(), tz: Some("Asia/Shanghai".into()) };

        let err = next_run_for_schedule(&schedule, from).unwrap_err().to_string();

        assert!(err.contains("No future occurrence"));
        assert!(err.contains("0 0 0 1 1 * 2020"));
    }

    #[test]
    fn validate_schedule_rejects_past_at_and_invalid_cron_timezone() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let past = Schedule::At { at: Utc.with_ymd_and_hms(2026, 6, 12, 9, 59, 59).unwrap() };
        let past_err = validate_schedule(&past, now).unwrap_err().to_string();
        assert!(past_err.contains("'at' must be in the future"));

        let invalid_tz =
            Schedule::Cron { expr: "0 9 * * *".into(), tz: Some("Mars/Olympus".into()) };
        let tz_err = validate_schedule(&invalid_tz, now).unwrap_err().to_string();
        assert!(tz_err.contains("Invalid IANA timezone"));
    }

    #[test]
    fn validate_schedule_accepts_future_at_and_positive_every() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let at = Schedule::At { at: Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 1).unwrap() };

        validate_schedule(&at, now).unwrap();
        validate_schedule(&Schedule::Every { every_ms: 1 }, now).unwrap();
    }

    #[test]
    fn validate_schedule_accepts_valid_cron() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "30 10 * * *".into(), tz: None };

        validate_schedule(&schedule, now).unwrap();
    }

    #[test]
    fn validate_schedule_rejects_cron_with_unsupported_field_count() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();
        let schedule = Schedule::Cron { expr: "0 10 * *".into(), tz: None };

        let err = validate_schedule(&schedule, now).unwrap_err().to_string();

        assert!(err.contains("expected 5, 6, or 7 fields"));
        assert!(err.contains("got 4"));
    }

    #[test]
    fn validate_schedule_rejects_at_equal_to_now() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();

        let err = validate_schedule(&Schedule::At { at: now }, now).unwrap_err().to_string();

        assert!(err.contains("'at' must be in the future"));
    }

    #[test]
    fn validate_schedule_rejects_zero_every() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap();

        let err = validate_schedule(&Schedule::Every { every_ms: 0 }, now).unwrap_err().to_string();

        assert!(err.contains("every_ms must be > 0"));
    }

    #[test]
    fn schedule_cron_expression_only_returns_cron_expressions() {
        let cron = Schedule::Cron { expr: "0 9 * * *".into(), tz: None };
        let at = Schedule::At { at: Utc.with_ymd_and_hms(2026, 6, 12, 10, 0, 0).unwrap() };
        let every = Schedule::Every { every_ms: 1000 };

        assert_eq!(schedule_cron_expression(&cron).as_deref(), Some("0 9 * * *"));
        assert!(schedule_cron_expression(&at).is_none());
        assert!(schedule_cron_expression(&every).is_none());
    }
}
