use super::*;
use crate::app::state::CronAddJobType;
use chrono::{FixedOffset, TimeZone};
fn now() -> DateTime<FixedOffset> {
    FixedOffset::east_opt(28800).unwrap().with_ymd_and_hms(2026, 12, 31, 23, 59, 0).unwrap()
}
#[test]
fn cron_templates_catalog_matches_reference() {
    let titles = [
        "每日 AI 新闻推送",
        "每日 5 个英语单词",
        "每日儿童睡前故事",
        "每周工作周报",
        "经典电影推荐",
        "历史上的今天",
        "每日一个为什么",
        "父母联系提醒",
        "体检预约提醒",
        "面试准备提醒",
        "会议前准备",
        "可爱萌宠手机壁纸",
    ];
    assert_eq!(CRON_TEMPLATES.len(), 12);
    for (t, title) in CRON_TEMPLATES.iter().zip(titles) {
        assert_eq!(t.title, title);
        assert!(!t.summary.is_empty());
        assert!(!t.prompt.is_empty());
    }
}
#[test]
fn cron_templates_draft_preserves_safe_defaults() {
    for t in CRON_TEMPLATES {
        let d = t.draft(now()).unwrap();
        assert_eq!(d.name, t.title);
        assert_eq!(d.job_type, CronAddJobType::Agent);
        assert!(
            !d.full_access
                && !d.wake
                && !d.task_pool
                && !d.delivery_enabled
                && !d.full_access_confirmation
        );
        assert!(
            d.command.is_empty()
                && d.project_path.is_empty()
                && d.model.is_empty()
                && d.model_provider.is_empty()
                && d.delivery_to.is_empty()
        );
        assert_eq!(d.session_target, "isolated");
        assert_eq!(d.agent, "main");
    }
}
#[test]
fn cron_templates_prompt_editor_matches_text() {
    for t in CRON_TEMPLATES {
        let d = t.draft(now()).unwrap();
        assert_eq!(d.prompt, t.prompt);
        assert_eq!(d.prompt_editor.text(), d.prompt);
    }
}
#[test]
fn cron_templates_one_shot_is_future_across_year_boundary() {
    for (id, hour) in [(CronTemplateId::Health, 7), (CronTemplateId::Meeting, 9)] {
        let d = CRON_TEMPLATES.iter().find(|t| t.id == id).unwrap().draft(now()).unwrap();
        assert_eq!(d.at, format!("2027-01-01T{hour:02}:00:00+08:00"));
        assert_eq!(d.schedule_kind, CronAddScheduleKind::At);
        assert!(DateTime::parse_from_rfc3339(&d.at).unwrap() > now());
    }
}
#[test]
fn cron_templates_custom_cron_preserves_expression() {
    for t in CRON_TEMPLATES {
        if let CronTemplateSchedule::Cron(expr) = t.schedule {
            let d = t.draft(now()).unwrap();
            assert_eq!(d.schedule, expr);
            assert_eq!(d.frequency, CronFrequency::from_expression(expr));
            if t.id == CronTemplateId::Interview {
                assert_eq!(d.frequency, CronFrequency::Advanced);
            }
        }
    }
}

#[test]
fn cron_templates_one_shot_rejects_invalid_and_ambiguous_local_time() {
    let template = CronTemplate {
        schedule: CronTemplateSchedule::TomorrowAt { hour: 2 },
        ..CRON_TEMPLATES[8]
    };
    let zone = chrono_tz::Europe::Berlin;
    for (month, day) in [(3, 28), (10, 24)] {
        let now = zone.with_ymd_and_hms(2026, month, day, 12, 0, 0).unwrap();
        assert!(template.draft(now).is_err());
    }
}
