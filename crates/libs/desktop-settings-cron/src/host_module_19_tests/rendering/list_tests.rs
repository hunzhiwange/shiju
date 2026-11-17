//! List acceptance uses the shared isolated tiny-skia harness and real mouse events.
use super::*;
use crate::state::CronListFilter;

fn list_fixture() -> Fixture {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.cron_settings = crate::job_test_fixture::list_state();
    app
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_list_summary_groups_and_direct_toggle() {
    if !isolated("list_tests::cron_list_summary_groups_and_direct_toggle") {
        return;
    }
    let mut renderer = renderer().await;
    let app = list_fixture();
    let size = Size::new(640, 700);
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "cron-list-baseline",
        mouse::Cursor::Unavailable,
    );
    assert_fits(positions.label("检查运行状态并生成摘要"), size);
    assert_fits(positions.label("全部 (1)"), size);
    assert!(positions.labels.iter().all(|(s, _)| !s.starts_with("全选")));
    let title = positions.label("每日巡检");
    assert!(
        matches!(click(&mut ui, &mut renderer, Point::new(title.x + title.width + 32.0, title.center_y())).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEnabledChanged(id, false))] if id == "list-job")
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("已暂停 (0)").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronListFilterChanged(CronListFilter::Paused))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("批量管理").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronBatchModeToggled(true))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_list_batch_footer_stays_fixed_and_empty_group() {
    if !isolated("list_tests::cron_list_batch_footer_stays_fixed_and_empty_group") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.cron_settings.batch_mode = true;
    app.cron_settings.jobs = (0..20)
        .map(|i| {
            let mut job = app.cron_settings.jobs[0].clone();
            job.id = format!("job-{i}");
            job
        })
        .collect();
    let size = Size::new(420, 740);
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let before = geometry(&mut ui, &renderer);
    let footer = before.label("全选 20 个任务");
    assert_fits(footer, size);
    let scroll = before.scroll.unwrap().0;
    assert!(footer.y >= scroll.y + scroll.height);
    assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
    assert_eq!(geometry(&mut ui, &renderer).label("全选 20 个任务"), footer);
    assert!(matches!(
        click(&mut ui, &mut renderer, footer.center()).as_slice(),
        [Message::Settings(SettingsMessage::CronJobsSelectAllToggled(true))]
    ));
    let disabled = before.label("删除已选(0)");
    assert!(click(&mut ui, &mut renderer, disabled.center()).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, before.label("取消批量管理").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronBatchModeToggled(false))]
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "cron-list-footer-420",
        mouse::Cursor::Unavailable,
    );
    drop(ui);
    app.cron_settings.list_filter = CronListFilter::Paused;
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    assert_fits(geometry(&mut ui, &renderer).label("当前分组暂无任务"), size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_list_controls_dispatch_correct_job_ids() {
    if !isolated("list_tests::cron_list_controls_dispatch_correct_job_ids") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.cron_settings.batch_mode = true;
    app.cron_settings.expanded_job_ids = vec!["list-job".into()];
    app.cron_settings.selected_job_ids = vec!["list-job".into()];
    let size = Size::new(1000, 1000);
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    for (label, expected) in [
        ("编辑", SettingsMessage::CronJobEditStarted("list-job".into())),
        ("历史", SettingsMessage::CronJobRunsOpen("list-job".into())),
        ("删除", SettingsMessage::CronJobDelete("list-job".into())),
        ("收起", SettingsMessage::CronJobDetailsToggled("list-job".into(), false)),
        ("启用已选(1)", SettingsMessage::CronSelectedJobsEnable),
        ("禁用已选(1)", SettingsMessage::CronSelectedJobsDisable),
        ("删除已选(1)", SettingsMessage::CronSelectedJobsDelete),
    ] {
        let messages = click(&mut ui, &mut renderer, positions.label(label).center());
        assert_eq!(format!("{messages:?}"), format!("{:?}", vec![Message::Settings(expected)]));
    }
    let title = positions.label("每日巡检");
    assert!(
        matches!(click(&mut ui, &mut renderer, Point::new(title.x - 20.0, title.center_y())).as_slice(),
        [Message::Settings(SettingsMessage::CronJobSelectionToggled(id, false))] if id == "list-job")
    );
    assert!(
        matches!(click(&mut ui, &mut renderer, Point::new(title.x + title.width + 32.0, title.center_y())).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEnabledChanged(id, false))] if id == "list-job")
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("已启用 (1)").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronListFilterChanged(CronListFilter::Enabled))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("新建定时任务").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronTabSelected(CronSettingsTab::Add))]
    ));
    drop(ui);
    app.cron_settings.expanded_job_ids.clear();
    let mut ui = scene(frame(view(&app)), &mut renderer, size);
    let details = geometry(&mut ui, &renderer).label("展开");
    assert!(matches!(click(&mut ui, &mut renderer, details.center()).as_slice(),
        [Message::Settings(SettingsMessage::CronJobDetailsToggled(id, true))] if id == "list-job"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wb_schedule_acceptance_cards_preserve_timezone_and_biweekly_rule() {
    if !isolated("list_tests::wb_schedule_acceptance_cards_preserve_timezone_and_biweekly_rule") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.cron_settings.jobs[0].expression = "0 30 9 * * *".into();
    app.cron_settings.jobs[0].timezone = Some("Asia/Shanghai".into());
    let mut biweekly = app.cron_settings.jobs[0].clone();
    biweekly.id = "biweekly-card".into();
    biweekly.name = Some("双周巡检".into());
    biweekly.schedule_kind = "biweekly".into();
    biweekly.expression.clear();
    biweekly.anchor_date = Some("2096-12-24".into());
    biweekly.weekdays = Some(vec![1, 5]);
    biweekly.hour = Some(9);
    biweekly.minute = Some(30);
    app.cron_settings.jobs.push(biweekly);
    let size = Size::new(760, 900);
    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme.clone();
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("wb-acceptance-card-zones-{theme:?}"),
            mouse::Cursor::Unavailable,
        );
        let positions = geometry(&mut ui, &renderer);
        for label in
            ["每天 09:30 · Asia/Shanghai", "每双周 [1, 5] 09:30 · 锚点 2096-12-24 · Asia/Shanghai"]
        {
            assert_fits(positions.label(label), size);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_templates_render_two_columns_and_select_template() {
    if !isolated("list_tests::cron_templates_render_two_columns_and_select_template") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.cron_settings.show_cron_templates = true;
    let size = Size::new(1000, 760);
    for theme in [Theme::Dark, Theme::Light] {
        let mut ui = scene(frame(view(&app)), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        let first = positions.label("每日 AI 新闻推送");
        let second = positions.label("每日 5 个英语单词");
        assert!((first.y - second.y).abs() < 1.0);
        assert!(second.x > first.x);
        capture(&mut ui, &mut renderer, &theme, size, "cron-templates", mouse::Cursor::Unavailable);
        assert!(matches!(
            click(&mut ui, &mut renderer, first.center()).as_slice(),
            [Message::Settings(SettingsMessage::CronTemplateSelected(
                crate::state::CronTemplateId::AiNews
            ))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_template_flow_cards_render_and_click_all_templates() {
    if !isolated("list_tests::cron_template_flow_cards_render_and_click_all_templates") {
        return;
    }
    let mut renderer = renderer().await;
    for width in [380, 700, 1100] {
        let size = Size::new(width, 1800);
        for theme in [Theme::Light, Theme::Dark] {
            let mut ui = scene(frame(templates::view()), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                "template-flow-cards",
                mouse::Cursor::Unavailable,
            );
            for template in crate::state::CRON_TEMPLATES {
                let bounds = positions.label(template.title);
                assert_fits(bounds, size);
                let messages = click(&mut ui, &mut renderer, bounds.center());
                assert!(matches!(messages.as_slice(),
                    [Message::Settings(SettingsMessage::CronTemplateSelected(id))] if *id == template.id));
                let mut app = list_fixture();
                for message in messages {
                    if let Message::Settings(event) = message {
                        drop(update(&mut app, event));
                    }
                }
                assert!(app.cron_settings.show_add_modal);
                assert_eq!(app.cron_settings.add_draft.prompt, template.prompt);
            }
        }
    }
}
