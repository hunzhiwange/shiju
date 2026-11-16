use super::*;
#[cfg(not(target_arch = "wasm32"))]
mod modal_render {
    use super::*;
    use crate::message::SettingsMessage;
    use crate::state::{CronScheduleDraft, CronScheduleRule as R};
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Event, Font, Rectangle, Renderer, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    #[derive(Default)]
    struct Geometry {
        labels: Vec<(String, Rectangle)>,
        scrolls: Vec<Rectangle>,
        inputs: Vec<Rectangle>,
    }
    impl iced::advanced::widget::Operation for Geometry {
        fn traverse(
            &mut self,
            children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
        ) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.into(), bounds));
        }
        fn text_input(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs.push(bounds);
        }
        fn scrollable(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: Rectangle,
            _: iced::Vector,
            _: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            self.scrolls.push(bounds);
        }
    }

    fn isolated(test: &str) -> bool {
        const CHILD: &str = "SHIJU_CRON_MODAL_RENDER_CHILD";
        if std::env::var_os(CHILD).is_some() {
            return true;
        }
        let home = tempfile::tempdir().unwrap();
        let config = config_ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        false
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wb_schedule_modal_renders_all_rules_and_fixed_actions() {
        if !isolated("wb_schedule_modal_renders_all_rules_and_fixed_actions") {
            return;
        }
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
                .await
                .unwrap();
        let mut app = test_app();
        app.desktop_locale = config_ui::DesktopLocale::SimplifiedChinese;
        let time = "09:30:00".parse().unwrap();
        let date = "2099-01-01".parse().unwrap();
        let rules = [
            R::At { date, time },
            R::Every { amount: 2, unit: crate::state::IntervalUnit::Hours },
            R::Daily { time },
            R::Weekly { weekdays: vec![1, 3], time },
            R::Biweekly { anchor: date, weekdays: vec![1, 5], time },
            R::Monthly { days: vec![1, 15, 31], time },
            R::Yearly { month: 2, day: 29, time },
            R::Advanced("*/5 * * * *".into()),
        ];
        let directory =
            std::env::var_os("SHIJU_CRON_MODAL_SCREENSHOTS").map(std::path::PathBuf::from);
        for (index, rule) in rules.iter().enumerate() {
            for editing in [false, true] {
                for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
                    for (size_name, size) in
                        [("narrow", Size::new(380, 680)), ("wide", Size::new(1100, 900))]
                    {
                        app.app_theme = theme.clone();
                        app.cron_settings.show_add_modal = !editing;
                        app.cron_settings.editing_job_id = editing.then(|| "render".into());
                        let draft = if editing {
                            &mut app.cron_settings.edit_draft
                        } else {
                            &mut app.cron_settings.add_draft
                        };
                        draft.schedule_draft = Some(CronScheduleDraft {
                            rule: rule.clone(),
                            timezone: "Asia/Shanghai".into(),
                            start_date: Some(Some(date)),
                            end_date: Some(Some("2099-12-31".parse().unwrap())),
                            ..Default::default()
                        });
                        draft.prompt = "检查项目并生成中文报告\n".repeat(40);
                        draft.prompt_editor =
                            iced::widget::text_editor::Content::with_text(&draft.prompt);
                        draft.preview = Some(Ok(gateway_cron::CronPreviewResponse {
                            normalized: Default::default(),
                            timezone: "Asia/Shanghai".into(),
                            candidates: (2..=if index == 0 { 2 } else { 6 })
                                .map(|day| format!("2099-01-{day:02}T01:30:00Z"))
                                .collect(),
                        }));
                        let mut ui = UserInterface::build(
                            with_edit_modal(
                                &app,
                                iced::widget::container(text("任务列表"))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .into(),
                            ),
                            Size::new(size.width as f32, size.height as f32),
                            Cache::new(),
                            &mut renderer,
                        );
                        let mut redraw_messages = Vec::new();
                        ui.update(
                            &[Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut redraw_messages,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&renderer, &mut geometry);
                        let save = geometry
                            .labels
                            .iter()
                            .find(|(label, _)| label == "保存")
                            .expect("fixed save button")
                            .1;
                        assert!(
                            save.y >= 0.0 && save.y + save.height <= size.height as f32,
                            "save clipped: {save:?}"
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("时区：Asia/Shanghai"))
                        );
                        assert!(!geometry.scrolls.is_empty());
                        assert_eq!(
                            geometry.labels.iter().any(|(label, _)| label == "有效期"),
                            index != 0
                        );
                        if index == 3 || index == 4 {
                            assert!(
                                geometry
                                    .labels
                                    .iter()
                                    .any(|(label, _)| label == "星期（至少选择一天）")
                            );
                        }
                        if index == 5 {
                            assert!(
                                geometry
                                    .labels
                                    .iter()
                                    .any(|(label, _)| label == "日期（至少选择一天）")
                            );
                        }
                        for input in &geometry.inputs {
                            assert!(
                                input.x >= 0.0 && input.x + input.width <= size.width as f32,
                                "input clipped: {input:?}"
                            );
                        }

                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                        assert_eq!(pixels.len(), (size.width * size.height * 4) as usize);
                        if let Some(directory) = &directory {
                            std::fs::create_dir_all(directory).unwrap();
                            ::image::save_buffer(
                                directory.join(format!(
                                    "rule-{index}-{}-{theme_name}-{size_name}.png",
                                    if editing { "edit" } else { "add" }
                                )),
                                &pixels,
                                size.width,
                                size.height,
                                ::image::ColorType::Rgba8,
                            )
                            .unwrap();
                        }
                        let mut ignored = Vec::new();
                        ui.update(
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -80.0 },
                            })],
                            mouse::Cursor::Available(geometry.scrolls[0].center()),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut ignored,
                        );
                        ui.update(
                            &[Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut redraw_messages,
                        );
                        let mut scrolled = Geometry::default();
                        ui.operate(&renderer, &mut scrolled);
                        assert_eq!(
                            save,
                            scrolled.labels.iter().find(|(label, _)| label == "保存").unwrap().1
                        );
                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        if index == 0 {
                            if let Some(directory) = &directory {
                                let pixels =
                                    renderer.screenshot(size, 1.0, theme.palette().background);
                                ::image::save_buffer(
                                    directory.join(format!(
                                        "scrolled-{}-{theme_name}-{size_name}.png",
                                        if editing { "edit" } else { "add" }
                                    )),
                                    &pixels,
                                    size.width,
                                    size.height,
                                    ::image::ColorType::Rgba8,
                                )
                                .unwrap();
                            }
                        }
                        let mut messages = Vec::new();
                        let point = save.center();
                        ui.update(
                            &[
                                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                            ],
                            mouse::Cursor::Available(point),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                        assert!(messages.iter().any(|m| matches!(
                            m,
                            Message::Settings(
                                SettingsMessage::CronAddSubmit | SettingsMessage::CronJobEditSave
                            )
                        )));
                    }
                }
            }
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn wb_schedule_feedback_renders_permissions_and_delivery() {
        if !isolated("wb_schedule_feedback_renders_permissions_and_delivery") {
            return;
        }
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
                .await
                .unwrap();
        let mut app = test_app();
        app.desktop_locale = config_ui::DesktopLocale::SimplifiedChinese;
        app.cron_settings.show_add_modal = true;
        let draft = &mut app.cron_settings.add_draft;
        draft.schedule_draft = Some(Default::default());
        draft.job_type = CronAddJobType::Agent;
        draft.delivery_enabled = true;
        draft.delivery_channel = "wecom".into();
        draft.delivery_to = "旧目标".into();
        for confirm in [false, true] {
            app.cron_settings.add_draft.full_access_confirmation = confirm;
            for theme in [Theme::Light, Theme::Dark] {
                for size in [Size::new(380, 680), Size::new(1100, 900)] {
                    app.app_theme = theme.clone();
                    let mut ui = UserInterface::build(
                        with_edit_modal(
                            &app,
                            iced::widget::container(text("任务列表"))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .into(),
                        ),
                        Size::new(size.width as f32, size.height as f32),
                        Cache::new(),
                        &mut renderer,
                    );
                    let mut messages = Vec::new();
                    ui.update(
                        &[Event::Window(iced::window::Event::RedrawRequested(
                            iced::time::Instant::now(),
                        ))],
                        mouse::Cursor::Unavailable,
                        &mut renderer,
                        &mut clipboard::Null,
                        &mut messages,
                    );
                    let mut geometry = Geometry::default();
                    ui.operate(&renderer, &mut geometry);
                    if confirm {
                        let label = geometry
                            .labels
                            .iter()
                            .find(|(label, _)| label == "确认授权")
                            .expect("confirmation button");
                        assert!(label.1.x >= 0.0 && label.1.x + label.1.width <= size.width as f32);
                        assert!(
                            label.1.y >= 0.0 && label.1.y + label.1.height <= size.height as f32
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("仅为当前 Agent"))
                        );
                    } else {
                        assert!(
                            geometry.labels.iter().any(|(label, _)| label.contains("默认受限："))
                        );
                        assert!(
                            geometry.labels.iter().any(|(label, _)| label.contains("ACP 未获授权"))
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("投递通道 wecom 未配置"))
                        );
                        assert!(geometry.labels.iter().any(|(label, _)| label == "通知配置"));
                        let scroll = geometry.scrolls[0];
                        ui.update(
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -100.0 },
                            })],
                            mouse::Cursor::Available(scroll.center()),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                    }
                }
            }
        }
    }
}
