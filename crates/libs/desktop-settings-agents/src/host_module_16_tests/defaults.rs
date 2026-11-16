use super::profile_update::{ProfileFixture, defaults_input, dispatch, validation_error};
use crate::{agents_update_message::AgentsMessage, state::DelegateAgentSettingsEntry};
use config_agent_profile::{AgentDefault, AgentDefaults, AgentRuntime};
#[test]
fn agent_defaults_form_switch_preserves_invalid_draft_and_prevents_save() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into()];
    let dispatch = |app: &mut ProfileFixture, message| {
        let _ = dispatch(app, message);
    };
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("custom".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    let defaults = AgentDefaults {
        permission: AgentDefault::Fixed(serde_json::json!({"*":"ask"})),
        ..Default::default()
    };
    dispatch(&mut app, AgentsMessage::ProfileDefaultsChanged(defaults.clone()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(!app.agents_settings.profile_saving);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().defaults, defaults);
    assert!(!app.agents_settings.entries.iter().any(|entry| entry.key == "custom"));
}

#[test]
fn agent_defaults_form_forced_binding_rejects_conflicting_default() {
    use config_agent_profile::AgentModelReference;
    let app = ProfileFixture::default();
    let mut draft = DelegateAgentSettingsEntry::from_config("custom", None);
    draft.force_model_selection = true;
    draft.provider = "provider".into();
    draft.model = "forced".into();
    draft.defaults.model = AgentDefault::Fixed(AgentModelReference {
        provider: "provider".into(),
        model: "other".into(),
    });
    assert!(validation_error(&app, &draft).unwrap().contains("强制模型绑定冲突"));
}

#[test]
fn agent_config_cleanup_single_model_group_prevents_conflict() {
    use config_agent_profile::AgentModelReference;
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let dispatch = |app: &mut ProfileFixture, message| {
        let _ = dispatch(app, message);
    };
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
    let model = AgentModelReference { provider: "provider".into(), model: "model".into() };
    let defaults =
        AgentDefaults { model: AgentDefault::Fixed(model.clone()), ..Default::default() };
    dispatch(&mut app, AgentsMessage::ProfileModelChanged(defaults.model));
    dispatch(&mut app, AgentsMessage::ForceModelSelectionToggled("main".into(), true));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert_eq!(draft.provider, model.provider);
    assert_eq!(draft.model, model.model);
    assert!(draft.force_model_selection);
    dispatch(&mut app, AgentsMessage::ProfileModelChanged(AgentDefault::Inherit));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert!(!draft.force_model_selection);
    assert!(draft.provider.is_empty() && draft.model.is_empty());
}

#[test]
fn agent_config_cleanup_strict_skill_defaults_reject_save_outside_boundary() {
    let app = ProfileFixture::default();
    let mut draft = app.agents_settings.entries[0].clone();
    draft.skill_policy = config_agent::DelegateCapabilityPolicy::Strict;
    draft.allowed_skills = vec![];
    draft.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    assert!(validation_error(&app, &draft).is_some());
    draft.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(validation_error(&app, &draft).is_none());
    let empty = crate::defaults::patch(&draft.defaults);
    assert_eq!(empty["skills"]["value"], serde_json::json!([]));
    draft.defaults.skills = AgentDefault::Inherit;
    assert_eq!(crate::defaults::patch(&draft.defaults)["skills"]["value"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_model_inheritance_renders_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = ProfileFixture::default();
        app.agents_settings.config_ready = true;
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));

        app.agents_settings.profile_modal.as_mut().unwrap().runtime =
            AgentRuntime::Acp { backend: "codex".into() };
        app.mcp_settings.inspect_loading = false;
        app.mcp_settings.inspect_error = Some("隔离目录加载失败".into());
        app.agents_settings.profile_modal.as_mut().unwrap().defaults.mcp =
            AgentDefault::Fixed(vec!["中文服务".into()]);
        let mut element: iced::Element<'_, crate::message::Event> =
            iced::widget::container(crate::defaults::view(
                defaults_input(&app, app.agents_settings.profile_modal.as_ref().unwrap()),
                app.agents_settings.profile_modal.as_ref().unwrap(),
            ))
            .padding(24)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(|theme: &Theme| {
                iced::widget::container::Style::default().background(theme.palette().background)
            })
            .into();
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(1100.0, 900.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        assert_eq!(node.bounds().size(), bounds);
        #[derive(Default)]
        struct Labels(Vec<String>);
        impl iced::advanced::widget::Operation for Labels {
            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
            ) {
                operate(self);
            }
            fn text(
                &mut self,
                _id: Option<&iced::advanced::widget::Id>,
                _bounds: Rectangle,
                text: &str,
            ) {
                self.0.push(text.to_owned());
            }
        }
        let mut labels = Labels::default();
        element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
        assert!(!labels.0.iter().any(|text| text == "默认权限"));
        assert!(!labels.0.iter().any(|text| text.contains("CLI 本地配置")));
        assert!(labels.0.iter().any(|text| text == "仅已声明会话模型能力的引擎可固定模型。"));
        assert!(labels.0.iter().any(|text| text == "强制使用所选模型"));
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-model-inheritance-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
