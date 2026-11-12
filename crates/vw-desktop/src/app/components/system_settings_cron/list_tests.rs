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
