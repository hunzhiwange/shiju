use desktop_apps_ui::aggregation::omo_state::*;
use vw_api_app_aggregation::*;
fn standard(revision: &str, current: Option<&str>) -> Snapshot {
    Snapshot::Standard(OmoState {
        revision: revision.into(),
        current_provider_id: current.map(Into::into),
        agents: None,
        categories: Some(Default::default()),
        other_fields: None,
    })
}

#[tokio::test]
async fn aggregation_t64_actual_text_rendering_in_both_themes() {
    use iced::{
        Font, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
        widget::Space,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        for variant in [Variant::Standard, Variant::Slim] {
            let mut form = Form::new(variant);
            form.busy = false;
            form.provider = "测试供应商／日本語".into();
            form.snapshot = Some(match variant {
                Variant::Standard => standard("r1", Some("当前供应商")),
                Variant::Slim => Snapshot::Slim(OmoSlimState {
                    revision: "s1".into(),
                    current_provider_id: Some("当前供应商".into()),
                    agents: Some(
                        serde_json::from_value(serde_json::json!({"助手":{"model":"示例模型"}}))
                            .unwrap(),
                    ),
                    other_fields: None,
                }),
            });
            let element = desktop_apps_ui::aggregation::omo_view::overlay(
                &form,
                Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
            );
            let size = Size::new(960, 800);
            let mut ui =
                UserInterface::build(element, Size::new(960.0, 800.0), Cache::new(), &mut renderer);
            let mut messages = Vec::new();
            ui.update(
                &[iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(messages.is_empty());
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 960 * 800 * 4);
            assert!(pixels.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2]));
            if let Some(directory) = std::env::var_os("SHIJU_T64_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(directory)
                        .join(format!("t64-{theme_name}-{variant:?}.png")),
                    &pixels,
                    960,
                    800,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
