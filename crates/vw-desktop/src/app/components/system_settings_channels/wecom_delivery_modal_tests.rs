#![cfg(not(target_arch = "wasm32"))]
use super::*;
use crate::app::message::settings;
use crate::app::state::WeComDeliveryMode;
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

#[tokio::test(flavor = "multi_thread")]
async fn wecom_delivery_modal_renders_narrow_chinese_and_qr_states() {
    const CHILD: &str = "SHIJU_WECOM_MODAL_RENDER_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let cfg = vw_config_types::ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&cfg).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::wecom_delivery_modal_renders_narrow_chinese_and_qr_states",
                    module_path!().split_once("::").unwrap().1
                ),
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
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
    app.channels_settings = Default::default();
    for (theme, width, locale, suffix) in [
        (Theme::Light, 360, DesktopLocale::SimplifiedChinese, "light-zh"),
        (Theme::Dark, 360, DesktopLocale::TraditionalChinese, "dark-tw"),
        (Theme::Dark, 640, DesktopLocale::English, "dark-en"),
    ] {
        app.desktop_locale = locale;
        for mode in [WeComDeliveryMode::Qr, WeComDeliveryMode::Manual, WeComDeliveryMode::Webhook] {
            drop(settings::update(
                &mut app,
                SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(mode)),
            ));
            assert!(app.channels_settings.wecom_delivery.is_some(), "entry must open modal");
            for (status, label) in [
                (None, "loading"),
                (Some(vw_gateway_client::ChannelQrStatusDto::Pending), "pending"),
                (Some(vw_gateway_client::ChannelQrStatusDto::Scanned), "scanned"),
                (Some(vw_gateway_client::ChannelQrStatusDto::Authorized), "authorized"),
                (Some(vw_gateway_client::ChannelQrStatusDto::Expired), "expired"),
                (Some(vw_gateway_client::ChannelQrStatusDto::Failed), "failed"),
            ] {
                app.channels_settings.qr_flow.status = status;
                app.channels_settings.qr_flow.credentials_ready =
                    status == Some(vw_gateway_client::ChannelQrStatusDto::Authorized);
                app.channels_settings.qr_flow.qrcode_image = if mode == WeComDeliveryMode::Qr
                    && matches!(
                        status,
                        Some(
                            vw_gateway_client::ChannelQrStatusDto::Pending
                                | vw_gateway_client::ChannelQrStatusDto::Scanned
                                | vw_gateway_client::ChannelQrStatusDto::Authorized
                        )
                    ) {
                    Some(iced::widget::image::Handle::from_rgba(16, 16, vec![255u8; 16 * 16 * 4]))
                } else {
                    None
                };
                app.channels_settings.wecom_delivery.as_mut().unwrap().error =
                    Some("保存失败：请检查网关连接 / Failed to save credentials".into());
                let size = Size::new(width, 740);
                let view = wecom_delivery_modal::view_overlays(
                    &app,
                    container(text("定时任务草稿 / Cron draft"))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into(),
                );
                let mut ui = UserInterface::build(
                    view,
                    Size::new(width as f32, 740.0),
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
                // Exercise the scrolled body so status/error text below a QR image is drawn too.
                if status.is_some() || mode != WeComDeliveryMode::Qr {
                    ui.update(
                        &[Event::Mouse(iced::mouse::Event::WheelScrolled {
                            delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -20.0 },
                        })],
                        mouse::Cursor::Available(iced::Point::new(width as f32 / 2.0, 420.0)),
                        &mut renderer,
                        &mut clipboard::Null,
                        &mut messages,
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
                let background = &pixels[pixels.len() - 4..];
                assert!(
                    pixels.chunks_exact(4).filter(|pixel| *pixel != background).count() > 10_000,
                    "modal must paint beyond the small background label"
                );
                if let Some(dir) = std::env::var_os("SHIJU_WECOM_MODAL_SCREENSHOTS") {
                    std::fs::create_dir_all(&dir).unwrap();
                    image::save_buffer(
                        std::path::PathBuf::from(dir)
                            .join(format!("wecom-{mode:?}-{label}-{suffix}.png")),
                        &pixels,
                        size.width,
                        size.height,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
                if mode != WeComDeliveryMode::Qr {
                    break;
                }
            }
        }
    }
}
