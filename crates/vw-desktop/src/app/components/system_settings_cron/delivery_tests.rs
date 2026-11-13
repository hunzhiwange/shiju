use super::*;
use iced::advanced::renderer::{Headless, Style};
use iced_runtime::user_interface::{Cache, UserInterface};

#[tokio::test]
async fn cron_wecom_form_renders_add_edit_and_configuration() {
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut app = App::new().0;
    app.cron_settings.show_add_modal = true;
    for editing in [false, true] {
        app.cron_settings.editing_job_id = editing.then(|| "fixture".into());
        for channel in ["wecom", "wecom_webhook"] {
            let draft = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            draft.delivery_enabled = true;
            draft.select_delivery_channel(channel.into());
            app.cron_settings.save_error = Some("配置失败，请检查连接后重试。".repeat(20));
            for theme in [Theme::Dark, Theme::Light] {
                for (width, height) in [(800, 720), (480, 600)] {
                    for config in [false, true] {
                        app.channels_settings.wecom_delivery =
                            config.then(|| crate::app::state::WeComDeliveryDraft {
                                mode: if channel == "wecom" {
                                    WeComDeliveryMode::Manual
                                } else {
                                    WeComDeliveryMode::Webhook
                                },
                                ..Default::default()
                            });
                        let base = super::super::view_overlays(
                            &app,
                            container(text("背景")).width(Length::Fill).height(Length::Fill).into(),
                        );
                        let view = crate::app::components::system_settings_channels::view_overlays(
                            &app, base,
                        );
                        let mut ui = UserInterface::build(
                            view,
                            iced::Size::new(width as f32, height as f32),
                            Cache::new(),
                            &mut renderer,
                        );
                        let mut messages = Vec::new();
                        ui.update(
                            &[iced::Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            iced::mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut iced::advanced::clipboard::Null,
                            &mut messages,
                        );
                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            iced::mouse::Cursor::Unavailable,
                        );
                        let pixels = renderer.screenshot(
                            iced::Size::new(width, height),
                            1.0,
                            theme.palette().background,
                        );
                        assert_eq!(pixels.len(), (width * height * 4) as usize);
                        let background = &pixels[pixels.len() - 4..];
                        assert!(
                            pixels.chunks_exact(4).filter(|pixel| *pixel != background).count()
                                > 10_000,
                            "弹窗必须实际绘制，不能只输出背景"
                        );
                        if let Some(dir) = std::env::var_os("SHIJU_TASK7_SCREENSHOTS") {
                            let name =
                                format!("{editing}-{channel}-{theme:?}-{width}-{config}.png");
                            ::image::save_buffer(
                                std::path::PathBuf::from(dir).join(name),
                                &pixels,
                                width,
                                height,
                                ::image::ColorType::Rgba8,
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
    }
}
