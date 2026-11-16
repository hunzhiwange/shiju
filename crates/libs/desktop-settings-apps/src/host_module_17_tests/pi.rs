use crate::aggregation::pi::Form;
use iced::widget::text_editor::Content;
use vw_api_app_aggregation::PiDirectoryState;

#[tokio::test]
async fn aggregation_t65_renders_dynamic_chinese_in_light_and_dark() {
    use iced::{
        Font, Length, Size, Theme,
        advanced::renderer::Headless,
        mouse,
        widget::{container, text},
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    let form = Form {
        snapshot: Some(PiDirectoryState {
            revision: "r1".into(),
            members: vec!["managed".into()],
            default_provider_id: Some("原生供应商".into()),
            default_model_id: Some("动态中文模型".into()),
        }),
        id: "managed".into(),
        name: "中文名称".into(),
        declaration: Content::with_text(r#"{"models":[{"id":"模型","name":"中文模型"}]}"#),
        ..Default::default()
    };
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    #[derive(Default)]
    struct Labels {
        text: Vec<String>,
        inputs: usize,
    }
    impl iced::advanced::widget::Operation for Labels {
        fn traverse(
            &mut self,
            children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
        ) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: iced::Rectangle, text: &str) {
            self.text.push(text.into());
        }
        fn text_input(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            _: iced::Rectangle,
            _: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs += 1;
        }
    }
    let mut images = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = UserInterface::build(
            desktop_apps_ui::aggregation::pi_view::overlay(
                &form,
                container(text("背景")).width(Length::Fill).height(Length::Fill).into(),
            ),
            Size::new(900., 800.),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        for label in ["Pi 模型目录", "原生供应商", "动态中文模型", "保存单条"] {
            assert!(labels.text.iter().any(|value| value == label), "missing modal label: {label}");
        }
        assert_eq!(labels.inputs, 2, "only provider ID and display name are editable text inputs");
        ui.draw(
            &mut renderer,
            &theme,
            &iced::advanced::renderer::Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(900, 800), 1., theme.palette().background);
        assert_eq!(pixels.len(), 900 * 800 * 4);
        let central_changes = (100..700)
            .flat_map(|y| (100..800).map(move |x| (y * 900 + x) * 4))
            .filter(|offset| pixels[*offset..*offset + 4] != pixels[..4])
            .count();
        assert!(
            central_changes > 5_000,
            "modal must paint inside the viewport, not just the background label"
        );
        image::save_buffer(
            format!("/tmp/shiju-t65-{}.png", if theme == Theme::Light { "light" } else { "dark" }),
            &pixels,
            900,
            800,
            image::ColorType::Rgba8,
        )
        .unwrap();
        images.push(pixels);
    }
    assert!(images[0] != images[1]);
}
