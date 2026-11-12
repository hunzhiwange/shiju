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

/// Exercise shaping and drawing, including responsive rows and the scrollable output modal.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_history_renders_chinese_long_lists_and_details() {
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Font, Renderer, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    const CHILD: &str = "SHIJU_CRON_HISTORY_RENDER_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let storage = tempfile::tempdir().unwrap();
        std::fs::write(
            storage.path().join("gateway-client-bootstrap.json"),
            r#"{"host":"[","servers":[]}"#,
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::cron_history_renders_chinese_long_lists_and_details",
                    module_path!().split_once("::").unwrap().1
                ),
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("VIBEWINDOW_CONFIG_DIR", storage.path())
            .env("VIBEWINDOW_WORKSPACE", storage.path())
            .current_dir(storage.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed; 0 failed"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut app = App::new().0;
    app.cron_settings.jobs = ["job-00000000", "job-00000001"]
        .map(|id| {
            serde_json::from_value(serde_json::json!({
                "id": id, "name": "中文重名任务", "next_run": "later", "enabled": true
            }))
            .unwrap()
        })
        .into();
    app.cron_settings.history.runs = (0..203)
        .map(|id| {
            let mut item = run(["running", "ok", "error"][id as usize % 3], id);
            item.name = Some(format!("中文任务{id}：{}", "很长的任务名称".repeat(12)));
            item.output =
                Some(format!("单次输出{id}\n{}", "错误说明：文件不存在 café 日本語\n".repeat(180)));
            item
        })
        .collect();
    app.cron_settings.history.truncated = true;
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        app.app_theme = theme.clone();
        for width in [420, 1100] {
            for scene in
                ["list", "picker", "details-first", "details-second", "missing", "error", "empty"]
            {
                app.cron_settings.history.task_picker_open = scene == "picker";
                app.cron_settings.history.selected_run = match scene {
                    "details-first" => Some(run_key(&app.cron_settings.history.runs[1])),
                    "details-second" => Some(run_key(&app.cron_settings.history.runs[2])),
                    "missing" => Some("removed".into()),
                    _ => None,
                };
                app.cron_settings.history.search =
                    if scene == "empty" { "不匹配".into() } else { String::new() };
                app.cron_settings.history.error =
                    (scene == "error").then(|| "本地服务暂不可用，请重试".into());
                let size = Size::new(width, 720);
                let content = with_details(&app, view(&app));
                let mut ui = UserInterface::build(
                    content,
                    Size::new(width as f32, 720.0),
                    Cache::new(),
                    &mut renderer,
                );
                ui.draw(
                    &mut renderer,
                    &theme,
                    &Style { text_color: theme.palette().text },
                    mouse::Cursor::Unavailable,
                );
                let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                assert_eq!(pixels.len(), width as usize * 720 * 4);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
                if let Some(directory) = std::env::var_os("SHIJU_CRON_HISTORY_SCREENSHOTS") {
                    ::image::save_buffer(
                        std::path::PathBuf::from(directory)
                            .join(format!("{theme_name}-{width}-{scene}.png")),
                        &pixels,
                        width,
                        720,
                        ::image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
                if matches!(scene, "list" | "details-first" | "details-second") {
                    let cursor =
                        mouse::Cursor::Available(iced::Point::new(width as f32 / 2.0, 400.0));
                    let mut messages = Vec::new();
                    ui.update(
                        &[iced::Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -400.0 },
                        })],
                        cursor,
                        &mut renderer,
                        &mut iced::advanced::clipboard::Null,
                        &mut messages,
                    );
                    ui.draw(
                        &mut renderer,
                        &theme,
                        &Style { text_color: theme.palette().text },
                        cursor,
                    );
                    let scrolled = renderer.screenshot(size, 1.0, theme.palette().background);
                    assert!(pixels != scrolled, "{theme_name}-{width}-{scene} must scroll");
                }
            }
        }
    }
}
