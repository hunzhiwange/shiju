use desktop_settings_channels::state::{
    ChannelsSettingsState, WeComDeliveryDraft, WeComDeliveryMode,
};
use desktop_settings_cron::{state::CronSettingsState, system_settings_cron as cron};
use iced::{
    Length, Theme,
    widget::{container, text},
};
#[derive(Clone)]
enum Message {
    None,
}

use iced::advanced::renderer::{Headless, Style};
use iced_runtime::user_interface::{Cache, UserInterface};

#[tokio::test]
async fn cron_wecom_form_renders_add_edit_and_configuration() {
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut state = CronSettingsState::default();
    let mut channels = ChannelsSettingsState::default();
    state.show_add_modal = true;
    for editing in [false, true] {
        state.editing_job_id = editing.then(|| "fixture".into());
        for channel in ["wecom", "wecom_webhook"] {
            let draft = if editing { &mut state.edit_draft } else { &mut state.add_draft };
            draft.delivery_enabled = true;
            draft.delivery_channel = channel.into();
            state.save_error = Some("配置失败，请检查连接后重试。".repeat(20));
            for theme in [Theme::Dark, Theme::Light] {
                for (width, height) in [(800, 720), (480, 600)] {
                    for config in [false, true] {
                        channels.wecom_delivery = config.then(|| WeComDeliveryDraft {
                            mode: if channel == "wecom" {
                                WeComDeliveryMode::Manual
                            } else {
                                WeComDeliveryMode::Webhook
                            },
                            ..Default::default()
                        });
                        let data = cron::ViewData {
                            cron_settings: &state,
                            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                            app_theme: &theme,
                            project_path: None,
                            recent_projects: &[],
                            agents: Vec::new(),
                            providers: Vec::new(),
                            delivery: Default::default(),
                            scheduler_error: None,
                            last_copied_code_hash: None,
                            help_content: |title, body, _, _| {
                                iced::widget::column![text(title), text(body)].into()
                            },
                        };
                        let base = cron::view_overlays(
                            &data,
                            container(text("背景")).width(Length::Fill).height(Length::Fill).into(),
                            |_| Message::None,
                        );
                        let view = desktop_settings_channels::wecom_delivery_modal::overlay(
                            desktop_settings_channels::ViewState {
                                channels_settings: &channels,
                                desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                                app_theme: &theme,
                            },
                            base,
                            |_| Message::None,
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
