//! Cron 类型转换的单元测试。
//!
//! 当前覆盖 `JobType` 的字符串解析边界，确保配置和外部输入只接受明确支持的任务
//! 类型。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use chrono::{TimeZone, Utc};

    use super::JobType;

    /// 验证已支持的任务类型可以大小写不敏感地解析。
    #[test]
    fn job_type_try_from_accepts_known_values_case_insensitive() {
        assert_eq!(JobType::try_from("shell").unwrap(), JobType::Shell);
        assert_eq!(JobType::try_from("SHELL").unwrap(), JobType::Shell);
        assert_eq!(JobType::try_from("agent").unwrap(), JobType::Agent);
        assert_eq!(JobType::try_from("AgEnT").unwrap(), JobType::Agent);
    }

    /// 验证空字符串和未知任务类型会被显式拒绝。
    #[test]
    fn job_type_try_from_rejects_invalid_values() {
        assert!(JobType::try_from("").is_err());
        assert!(JobType::try_from("unknown").is_err());
    }

    #[test]
    fn job_type_into_static_str_matches_serialized_names() {
        let shell: &'static str = JobType::Shell.into();
        let agent: &'static str = JobType::Agent.into();

        assert_eq!(shell, "shell");
        assert_eq!(agent, "agent");
    }

    #[test]
    fn job_type_default_and_json_use_shell_lowercase() {
        assert_eq!(JobType::default(), JobType::Shell);
        assert_eq!(serde_json::to_string(&JobType::Shell).unwrap(), r#""shell""#);
        assert_eq!(serde_json::from_str::<JobType>(r#""agent""#).unwrap(), JobType::Agent);
    }

    #[test]
    fn session_target_parse_and_as_str_are_case_insensitive() {
        assert_eq!(SessionTarget::default(), SessionTarget::Isolated);
        assert_eq!(SessionTarget::parse("main"), SessionTarget::Main);
        assert_eq!(SessionTarget::parse("MAIN").as_str(), "main");
        assert_eq!(SessionTarget::parse("isolated"), SessionTarget::Isolated);
        assert_eq!(SessionTarget::parse("unknown").as_str(), "isolated");
        assert_eq!(serde_json::to_string(&SessionTarget::Main).unwrap(), r#""main""#);
    }

    #[test]
    fn schedule_json_covers_all_variants_and_cron_default_tz() {
        let cron: Schedule =
            serde_json::from_str(r#"{"kind":"cron","expr":"0 0 * * * *"}"#).unwrap();
        assert_eq!(cron, Schedule::Cron { expr: "0 0 * * * *".to_string(), tz: None });

        let at = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        let at_json = serde_json::to_value(Schedule::At { at }).unwrap();
        assert_eq!(at_json["kind"], "at");
        assert_eq!(at_json["at"], "2026-01-02T03:04:05Z");

        let every_json = serde_json::to_value(Schedule::Every { every_ms: u64::MAX }).unwrap();
        assert_eq!(every_json["kind"], "every");
        assert_eq!(every_json["every_ms"], u64::MAX);
    }

    #[test]
    fn delivery_config_defaults_and_deserialize_defaults_best_effort() {
        let default = DeliveryConfig::default();
        assert_eq!(default.mode, "none");
        assert!(default.channel.is_none());
        assert!(default.to.is_none());
        assert!(default.best_effort);

        let parsed: DeliveryConfig = serde_json::from_str(r#"{"mode":"announce"}"#).unwrap();
        assert_eq!(parsed.mode, "announce");
        assert!(parsed.channel.is_none());
        assert!(parsed.to.is_none());
        assert!(parsed.best_effort);
    }

    #[test]
    fn delivery_config_deserializes_explicit_false_best_effort() {
        let parsed: DeliveryConfig = serde_json::from_str(
            r#"{"mode":"channel","channel":"telegram","to":"chat","best_effort":false}"#,
        )
        .unwrap();

        assert_eq!(parsed.mode, "channel");
        assert_eq!(parsed.channel.as_deref(), Some("telegram"));
        assert_eq!(parsed.to.as_deref(), Some("chat"));
        assert!(!parsed.best_effort);
    }

    #[test]
    fn normalize_fallbacks_trims_empty_values_and_duplicates() {
        let normalized = normalize_fallbacks(vec![
            " gpt-4.1 ".into(),
            "".into(),
            "gpt-4.1".into(),
            "claude".into(),
            "  ".into(),
        ]);

        assert_eq!(normalized, vec!["gpt-4.1".to_string(), "claude".to_string()]);
    }

    #[test]
    fn normalize_fallbacks_preserves_case_sensitive_distinct_values() {
        let normalized =
            normalize_fallbacks(vec!["model".into(), "MODEL".into(), " model ".into()]);

        assert_eq!(normalized, vec!["model".to_string(), "MODEL".to_string()]);
    }

    #[test]
    fn cron_job_json_round_trips_all_type_fields() {
        let now = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        let job = CronJob {
            id: "job-1".to_string(),
            expression: "0 0 * * * *".to_string(),
            schedule: Schedule::Cron {
                expr: "0 0 * * * *".to_string(),
                tz: Some("Asia/Shanghai".to_string()),
            },
            command: "echo ok".to_string(),
            prompt: Some("summarize".to_string()),
            name: Some("Nightly".to_string()),
            job_type: JobType::Agent,
            session_target: SessionTarget::Main,
            model: Some("model-a".to_string()),
            agent: Some("agent-a".to_string()),
            acp_agent: Some("acp-a".to_string()),
            project_path: Some("/tmp/project".to_string()),
            wake: true,
            fallbacks: vec!["fallback-a".to_string()],
            full_access: true,
            task_pool: true,
            enabled: true,
            delivery: DeliveryConfig {
                mode: "announce".to_string(),
                channel: Some("telegram".to_string()),
                to: Some("me".to_string()),
                best_effort: false,
            },
            delete_after_run: true,
            created_at: now,
            next_run: now,
            last_run: Some(now),
            last_status: Some("success".to_string()),
            last_output: Some("done".to_string()),
        };

        let parsed: CronJob = serde_json::from_value(serde_json::to_value(&job).unwrap()).unwrap();

        assert_eq!(parsed.id, "job-1");
        assert_eq!(parsed.job_type, JobType::Agent);
        assert_eq!(parsed.session_target, SessionTarget::Main);
        assert_eq!(parsed.delivery.best_effort, false);
        assert_eq!(parsed.fallbacks, vec!["fallback-a"]);
        assert!(parsed.full_access);
        assert!(parsed.task_pool);
        assert_eq!(parsed.last_run, Some(now));
    }

    #[test]
    fn cron_run_json_round_trips_optional_fields() {
        let started_at = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        let run = CronRun {
            id: 7,
            job_id: "job-1".to_string(),
            started_at,
            finished_at: started_at + chrono::Duration::milliseconds(250),
            status: "success".to_string(),
            output: Some("ok".to_string()),
            duration_ms: Some(250),
        };

        let parsed: CronRun = serde_json::from_value(serde_json::to_value(&run).unwrap()).unwrap();

        assert_eq!(parsed.id, 7);
        assert_eq!(parsed.job_id, "job-1");
        assert_eq!(parsed.started_at, started_at);
        assert_eq!(parsed.status, "success");
        assert_eq!(parsed.output.as_deref(), Some("ok"));
        assert_eq!(parsed.duration_ms, Some(250));
    }

    #[test]
    fn cron_job_patch_default_leaves_all_fields_unset() {
        let patch = CronJobPatch::default();

        assert!(patch.job_type.is_none());
        assert!(patch.schedule.is_none());
        assert!(patch.command.is_none());
        assert!(patch.prompt.is_none());
        assert!(patch.name.is_none());
        assert!(patch.enabled.is_none());
        assert!(patch.delivery.is_none());
        assert!(patch.model.is_none());
        assert!(patch.session_target.is_none());
        assert!(patch.delete_after_run.is_none());
        assert!(patch.agent.is_none());
        assert!(patch.acp_agent.is_none());
        assert!(patch.project_path.is_none());
        assert!(patch.wake.is_none());
        assert!(patch.fallbacks.is_none());
        assert!(patch.full_access.is_none());
        assert!(patch.task_pool.is_none());
    }
}
