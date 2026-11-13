//! List acceptance uses the shared isolated tiny-skia harness and real mouse events.
use super::*;
use crate::app::components::system_settings::SystemTab;
use crate::app::state::CronListFilter;

fn list_fixture() -> App {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    app.cron_settings.jobs_loading = false;
    let mut job = super::super::system_settings_cron_tests::job("list-job");
    job.name = Some("每日巡检".into());
    job.command = "检查运行状态并生成摘要".into();
    app.cron_settings.jobs = vec![job];
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
        matches!(click(&mut ui, &mut renderer, Point::new(positions.scroll.unwrap().0.width - 40.0, title.center_y())).as_slice(),
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
async fn cron_list_fills_available_content_width() {
    if !isolated("list_tests::cron_list_fills_available_content_width") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.show_system_settings = true;
    app.cron_settings.batch_mode = true;
    let mut widths = Vec::new();
    for width in [1280, 1920] {
        app.system_settings_tab = SystemTab::Cron;
        let size = Size::new(width, 900);
        let mut ui =
            scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        let scroll = positions.scroll.unwrap().0;
        let title = positions.label("定时任务配置");
        assert!(scroll.x >= 300.0 && scroll.x < 350.0, "{scroll:?}");
        assert!(scroll.x + scroll.width > width as f32 - 50.0, "{scroll:?}");
        assert!((title.x - scroll.x).abs() < 15.0);
        let heading = positions.label("当前任务");
        assert!((heading.center_y() - positions.label("新增任务").center_y()).abs() < 3.0);
        let frequency = positions.label("每天 09:00 · UTC");
        let next =
            positions.labels.iter().find(|(label, _)| label.starts_with("下次运行 ·")).unwrap().1;
        assert!((frequency.center_y() - next.center_y()).abs() < 0.5);
        let delete = positions.label("删除已选(0)");
        assert!(delete.x + delete.width > width as f32 - 60.0);
        assert!(positions.label("每日巡检").width > width as f32 - 500.0);
        widths.push(scroll.width);
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("cron-list-full-{width}"),
            mouse::Cursor::Unavailable,
        );
        drop(ui);
        app.system_settings_tab = SystemTab::Agents;
        app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Heartbeat;
        let mut ui =
            scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
        assert!(
            geometry(&mut ui, &renderer).label("开启周期心跳").x
                > if width == 1920 { 450.0 } else { 300.0 }
        );
    }
    assert!(widths[1] - widths[0] > 600.0);
}

fn assert_single_content_scroll(positions: &Geometry) {
    let content_left = positions.label("定时任务配置").x;
    assert_eq!(
        positions.scrolls.iter().filter(|r| r.x + r.width > content_left).count(),
        1,
        "Cron content has one scroll viewport; sidebar navigation scrolls independently"
    );
}

// Every scene goes through redraw, draw and screenshot, including the hostile text fixture.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_list_render_theme_and_width_matrix() {
    if !isolated("list_tests::cron_list_render_theme_and_width_matrix") {
        return;
    }
    let mut renderer = renderer().await;
    for theme in [Theme::Dark, Theme::Light] {
        let mut widths = Vec::new();
        for (width, height) in [(1920, 1080), (1280, 800), (760, 620)] {
            let mut app = list_fixture();
            app.app_theme = theme.clone();
            app.show_system_settings = true;
            app.system_settings_tab = SystemTab::Cron;
            app.cron_settings.batch_mode = true;
            let mut job = app.cron_settings.jobs[0].clone();
            job.name = Some("中文长标题巡检任务🧭".repeat(12));
            job.command = format!("检查状态\n{}\n输出摘要 ✅", "UnbrokenEnglishText".repeat(30));
            job.next_run = "invalid-time".into();
            job.last_output = Some(String::new());
            app.cron_settings.jobs = vec![job];
            let size = Size::new(width, height);
            for state in
                ["zero", "partial", "all", "paused", "group-empty", "empty", "loading", "error"]
            {
                app.cron_settings.selected_job_ids = match state {
                    "partial" | "all" => vec!["list-job".into()],
                    _ => Vec::new(),
                };
                app.cron_settings.jobs[0].enabled = state != "paused";
                app.cron_settings.list_filter = if state == "group-empty" {
                    CronListFilter::Paused
                } else {
                    CronListFilter::All
                };
                app.cron_settings.jobs_loading = state == "loading";
                app.cron_settings.save_error =
                    (state == "error").then(|| "中文错误反馈：示例错误".into());
                let saved = app.cron_settings.jobs.clone();
                if state == "empty" {
                    app.cron_settings.jobs.clear();
                }
                if state == "partial" {
                    let mut other = saved[0].clone();
                    other.id = "other-job".into();
                    other.name = None;
                    app.cron_settings.jobs.push(other);
                }
                let mut ui =
                    scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
                let positions = geometry(&mut ui, &renderer);
                assert_single_content_scroll(&positions);
                let scroll = positions.scroll.unwrap().0;
                let title = positions.label("定时任务配置");
                let group =
                    positions.labels.iter().find(|(s, _)| s.starts_with("全部 (")).unwrap().1;
                assert!((title.x - scroll.x).abs() <= 1.0);
                assert!((group.x - 14.0 - scroll.x).abs() <= 1.0);
                // Sidebar, divider and the shared 7px scrollbar gutters.
                assert!(
                    (scroll.x - ((width as f32 * 0.24).clamp(112.0, 300.0) + 8.0)).abs() <= 1.0,
                    "{scroll:?}"
                );
                assert!(
                    (scroll.x + scroll.width - (width as f32 - 7.0)).abs() <= 1.0,
                    "{scroll:?}"
                );
                let footer =
                    positions.labels.iter().find(|(s, _)| s.starts_with("全选 ")).unwrap().1;
                assert_fits(footer, size);
                assert!(footer.y >= scroll.y + scroll.height);
                let delete =
                    positions.labels.iter().find(|(s, _)| s.starts_with("删除已选(")).unwrap().1;
                assert_fits(delete, size);
                if state == "zero" {
                    widths.push(scroll.width);
                    assert!(
                        positions.containers.iter().any(|r| (r.x - scroll.x - 7.0).abs() <= 1.0
                            && (r.width - scroll.width + 14.0).abs() <= 1.0),
                        "card keeps both 7px scrollbar gutters"
                    );
                }
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("matrix-{theme:?}-{width}-{state}"),
                    mouse::Cursor::Unavailable,
                );
                drop(ui);
                app.cron_settings.jobs = saved;
            }
        }
        assert!((widths[0] - widths[1] - 640.0).abs() <= 1.0);
        let mut app = list_fixture();
        app.cron_settings.jobs[0].name = None;
        app.cron_settings.jobs[0].job_type = "agent".into();
        app.cron_settings.jobs[0].prompt = Some("多行提示词🧭\n检查中文字体\n整理输出".into());
        let size = Size::new(480, 620);
        let mut ui = scene(frame(job_row(&app, &app.cron_settings.jobs[0])), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_fits(positions.label("展开"), size);
        assert!(positions.containers.iter().any(|r| (r.width - 480.0).abs() <= 1.0));
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("card-480-{theme:?}"),
            mouse::Cursor::Unavailable,
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cron_list_bulk_footer_stays_visible_when_scrolled() {
    if !isolated("list_tests::cron_list_bulk_footer_stays_visible_when_scrolled") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Cron;
    app.cron_settings.batch_mode = true;
    let base = app.cron_settings.jobs[0].clone();
    app.cron_settings.jobs = (0..30)
        .map(|i| {
            let mut job = base.clone();
            job.id = format!("job-{i}");
            job.name = Some(format!("巡检任务 {i}"));
            job
        })
        .collect();
    for size in [Size::new(760, 620), Size::new(1920, 1080)] {
        let mut ui =
            scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
        let before = geometry(&mut ui, &renderer);
        assert_single_content_scroll(&before);
        let scroll = before.scroll.unwrap().0;
        for _ in 0..100 {
            assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
        }
        let after = geometry(&mut ui, &renderer);
        let translation = after.scroll.unwrap().1;
        assert!(translation.y > 1000.0);
        assert!(scroll.contains(after.label("巡检任务 29").center() - translation));
        for label in ["定时任务配置", "全部 (30)", "全选 30 个任务", "删除已选(0)"]
        {
            assert_eq!(before.label(label), after.label(label));
            assert_fits(after.label(label), size);
        }
        assert!(after.label("全选 30 个任务").y >= scroll.y + scroll.height);
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("scroll-bottom-{}", size.width),
            mouse::Cursor::Unavailable,
        );
    }
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
        matches!(click(&mut ui, &mut renderer, Point::new(positions.scroll.unwrap().0.width - 40.0, title.center_y())).as_slice(),
        [Message::Settings(SettingsMessage::CronJobEnabledChanged(id, false))] if id == "list-job")
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("已启用 (1)").center()).as_slice(),
        [Message::Settings(SettingsMessage::CronListFilterChanged(CronListFilter::Enabled))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, positions.label("新增任务").center()).as_slice(),
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
async fn cron_list_tabs_and_modals_preserve_width_and_scroll() {
    if !isolated("list_tests::cron_list_tabs_and_modals_preserve_width_and_scroll") {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = list_fixture();
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Cron;
    for size in [Size::new(760, 620), Size::new(1920, 1080)] {
        let mut left = None;
        for tab in [CronSettingsTab::Jobs, CronSettingsTab::Config, CronSettingsTab::Scheduler] {
            app.cron_settings.active_tab = tab;
            let mut ui =
                scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
            let before = geometry(&mut ui, &renderer);
            let title = before.label("定时任务配置");
            assert!((title.x - *left.get_or_insert(title.x)).abs() <= 1.0);
            let content_width = size.width as f32 - title.x - 7.0;
            assert!(before.containers.iter().any(|r| (r.x - title.x).abs() <= 1.0 && (r.width - content_width).abs() <= 1.0), "Cron tab must fill the content area");
            assert_single_content_scroll(&before);
            capture(
                &mut ui,
                &mut renderer,
                &Theme::Light,
                size,
                &format!("tab-{tab:?}-{}", size.width),
                mouse::Cursor::Unavailable,
            );
        }
        app.cron_settings.active_tab = CronSettingsTab::Jobs;
        for modal in ["add", "edit", "history"] {
            app.cron_settings.show_add_modal = modal == "add";
            app.cron_settings.editing_job_id = (modal == "edit").then(|| "list-job".into());
            app.cron_settings.runs_modal_job_id = (modal == "history").then(|| "list-job".into());
            app.cron_settings.edit_draft.name = "中文动态任务 🧭".into();
            app.cron_settings.runs_modal_error = Some("中文历史错误：本地示例".into());
            let mut ui =
                scene(crate::app::components::system_settings::view(&app), &mut renderer, size);
            let before = geometry(&mut ui, &renderer);
            if matches!(modal, "add" | "edit") {
                let save = before.label("保存");
                assert_fits(save, size);
                let scroll = before.scroll.unwrap().0;
                assert!(wheel(&mut ui, &mut renderer, scroll.center()).is_empty());
                assert_eq!(geometry(&mut ui, &renderer).label("保存"), save);
                assert!(matches!(
                    click(&mut ui, &mut renderer, before.label("取消").center()).as_slice(),
                    [Message::Settings(SettingsMessage::CronJobEditCanceled)]
                ));
            } else {
                assert_fits(before.label("执行历史"), size);
                assert_fits(before.label("中文历史错误：本地示例"), size);
            }
            let expected_width: f32 = if modal == "history" { 780.0 } else { 740.0 };
            assert!(
                before
                    .containers
                    .iter()
                    .any(|r| (r.width - expected_width.min(size.width as f32 - 24.0)).abs() <= 1.0)
            );
            capture(
                &mut ui,
                &mut renderer,
                &Theme::Dark,
                size,
                &format!("modal-{modal}-{}", size.width),
                mouse::Cursor::Unavailable,
            );
        }
        app.cron_settings.show_add_modal = false;
        app.cron_settings.editing_job_id = None;
        app.cron_settings.runs_modal_job_id = None;
    }
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
            [Message::Settings(SettingsMessage::CronTemplateSelected(0))]
        ));
    }
}
