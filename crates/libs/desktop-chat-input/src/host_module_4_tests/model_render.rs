//! 中文模型与 ACP 来源控件的真实渲染路径。
use crate::input_panel::{acp_options, model_views};
use desktop_settings_providers::state::{ModelSettingsState, ModelSummary, ProviderModelsSummary};
use iced::{Element, Length};
use vw_api_acp_model::{acp_model_source::AcpModelSource, acp_session::AcpSessionOptions};
#[derive(Debug, Clone)]
enum Event {
    Model(model_views::Action),
    #[allow(dead_code)]
    Acp(acp_options::Action),
    Agent,
}
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
#[derive(Default)]
struct Fixture {
    settings: ModelSettingsState,
    label: String,
    model: String,
    source: AcpModelSource,
    options: AcpSessionOptions,
    open: bool,
    pending: bool,
    locale: config_ui::DesktopLocale,
}
impl Fixture {
    fn state(&self) -> acp_options::OptionsState<'_> {
        acp_options::OptionsState {
            i18n: desktop_i18n::I18n::new(self.locale),
            acp_agent: Some("claude"),
            acp_options: &self.options,
            acp_model_source: self.source,
            is_requesting: false,
            acp_options_pending: self.pending,
            source_pending: false,
            auto_model: false,
            model: &self.model,
            full_access_enabled: false,
            acp_rebuild_required: false,
        }
    }
    fn source(&self) -> Element<'_, Event> {
        acp_options::source_controls(
            &self.state(),
            false,
            iced::widget::text("模型来源").into(),
            Event::Acp,
        )
    }
    fn popover(&self) -> Element<'_, Event> {
        model_views::model_popover_content(
            model_views::ModelPopoverInput {
                settings: &self.settings,
                i18n: desktop_i18n::I18n::new(self.locale),
                cli_local: self.source == AcpModelSource::CliLocal,
                started: false,
                can_select: model_views::can_select_model(&self.state(), false),
                is_acp: true,
                input_model: self.model.clone(),
            },
            false,
            &self.model,
            self.source(),
            Event::Model,
        )
    }
    fn modal<'a>(&'a self, base: Element<'a, Event>) -> Element<'a, Event> {
        if !self.open {
            return base;
        }
        let permission = crate::input_panel::send_controls::acp_permission_modes(false, false)
            .map(|_| Event::Agent);
        let access = acp_options::access_status(&self.state(), permission);
        acp_options::with_modal(
            &self.state(),
            i18n(),
            false,
            base,
            access,
            self.source(),
            Event::Acp,
        )
    }
}
fn model(id: &str, name: &str, enabled: bool) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: serde_json::json!({}),
    }
}

fn provider(id: &str, name: &str, models: Vec<ModelSummary>) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: name.to_string(), models }
}

#[tokio::test(flavor = "multi_thread")]
async fn acp_model_source_ui_controls_render_chinese_labels_in_both_themes() {
    use iced::advanced::Renderer as _;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let mut app = Fixture::default();
    app.settings.providers =
        vec![provider("custom", "自定义供应商", vec![model("model", "中文模型名称", true)])];
    let runtime = &mut app;
    runtime.label = "中文智能体 · Claude".into();

    runtime.model = "custom/model".into();

    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    for theme in [Theme::Light, Theme::Dark] {
        for (source, collapsed) in [
            (AcpModelSource::SessionModel, false),
            (AcpModelSource::SessionModel, true),
            (AcpModelSource::CliLocal, false),
        ] {
            if collapsed {
                app.settings.input_collapsed_provider_ids.insert("custom".into());
            } else {
                app.settings.input_collapsed_provider_ids.clear();
            }
            app.source = source;
            for modal in [false, true] {
                app.open = modal;
                app.options.model_source_capabilities = vec![
                    vw_api_acp_model::acp_model_source::AcpModelSourceCapability::Unsupported {
                        source: AcpModelSource::SessionModel,
                        reason: vw_api_acp_model::acp_model_source::AcpModelSourceRejectionReason::UnsupportedCli,
                    },
                ];
                let mut element: iced::Element<'_, Event> = iced::widget::container(
                    iced::widget::column![
                        crate::input_panel::panel::session_tool_tab_toggle(
                            desktop_chat_tools::state::SessionToolSelectorTab::Agent,
                            false,
                            app.label.clone(),
                            Some("claude"),
                            Event::Agent,
                        ),
                        model_views::model_toggle_button(
                            i18n(),
                            &app.settings.providers,
                            app.source == AcpModelSource::CliLocal,
                            false,
                            "custom/model",
                            true,
                            Event::Model
                        ),
                        app.popover(),
                    ]
                    .spacing(12),
                )
                .padding(16)
                .style(|theme: &Theme| {
                    iced::widget::container::background(theme.palette().background)
                })
                .into();
                let base = iced::widget::container(element)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
                element = app.modal(base);
                let bounds = Size::new(640.0, 780.0);
                renderer.reset(Rectangle::with_size(bounds));
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, bounds),
                );
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style { text_color: theme.palette().text },
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &Rectangle::with_size(bounds),
                );
                let pixels = renderer.screenshot(Size::new(640, 780), 1.0, Color::TRANSPARENT);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
                let directory = std::env::temp_dir().join("shiju-input-agent-model-render");
                std::fs::create_dir_all(&directory).unwrap();
                ::image::RgbaImage::from_raw(640, 780, pixels)
                    .unwrap()
                    .save(directory.join(format!(
                        "{theme:?}-{source:?}-modal-{modal}-collapsed-{collapsed}.png"
                    )))
                    .unwrap();
            }
        }
    }
}

#[path = "model_results.rs"]
mod model_results;
