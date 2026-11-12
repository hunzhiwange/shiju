use super::*;
use vw_gateway_client::CronHistoryRunDto;
fn run(status: &str, id: i64) -> CronHistoryRunDto {
    CronHistoryRunDto {
        id: Some(id),
        execution_id: None,
        job_id: "中文任务".into(),
        name: Some("备份资料".into()),
        status: status.into(),
        started_at: "2026-11-12T01:02:03Z".into(),
        finished_at: None,
        duration_ms: Some(1234),
        output: Some(format!("输出{id}")),
    }
}
#[test]
fn cron_history_rows_format_three_states() {
    assert_eq!(status_label("running"), "◌ 运行中");
    assert_eq!(status_label("ok"), "成功");
    assert_eq!(status_label("error"), "失败");
    assert_eq!(status_label("other"), "未知状态（other）");
    assert_eq!(duration_label(&run("running", 1)), "—");
    assert_eq!(duration_label(&run("ok", 1)), "1.234 秒");
    let mut invalid = run("ok", 1);
    invalid.duration_ms = Some(-1);
    assert_eq!(duration_label(&invalid), "耗时不可用");
    assert!(absolute_time("bad").contains("时间无效"));
}
#[test]
fn cron_history_type_options_are_local_only() {
    assert_eq!(TYPE_OPTIONS.map(|v| v.to_string()), ["全部类型", "本地"]);
}
#[test]
fn cron_history_details_select_exact_run() {
    let first = run("ok", 1);
    let second = run("ok", 2);
    let mut state = crate::app::state::CronHistoryState::default();
    state.runs = vec![first.clone(), second.clone()];
    state.selected_run = Some(run_key(&second));
    assert_eq!(selected_run(&state).unwrap().output.as_deref(), Some("输出2"));
    state.runs.remove(1);
    assert!(selected_run(&state).is_none());
}

#[test]
fn cron_history_components_build_real_rows_and_missing_details() {
    let mut app = App::new().0;
    app.cron_settings.history.runs = vec![run("running", 1), run("ok", 2), run("error", 3)];
    app.cron_settings.history.task_picker_open = true;
    app.cron_settings.history.search_open = true;
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme;
        let _ = view(&app);
        for run in &app.cron_settings.history.runs {
            let _ = record_row(run, true);
            let _ = record_row(run, false);
        }
        app.cron_settings.history.selected_run = Some(run_key(&app.cron_settings.history.runs[1]));
        let _ = with_details(&app, text("定时任务").into());
        app.cron_settings.history.selected_run = Some("已移除".into());
        let _ = with_details(&app, text("定时任务").into());
    }
}
