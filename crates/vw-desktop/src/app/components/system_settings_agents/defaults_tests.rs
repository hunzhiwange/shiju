use crate::app::state::DelegateAgentSettingsEntry;
use vw_config_types::agent::{AgentDefault, AgentDefaults, AgentDefinitionConfig, AgentRuntime};

#[test]
fn agent_defaults_form_roundtrips_explicit_empty_lists() {
    let config = AgentDefinitionConfig {
        defaults: AgentDefaults {
            skills: AgentDefault::Fixed(vec![]),
            mcp: AgentDefault::Fixed(vec![]),
            ..Default::default()
        },
        ..Default::default()
    };
    let entry = DelegateAgentSettingsEntry::from_config("custom", Some(config));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("agent.json");
    std::fs::write(&path, serde_json::to_vec(&entry.defaults).unwrap()).unwrap();
    let wire = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let restored: AgentDefaults = serde_json::from_value(wire).unwrap();
    assert_eq!(restored.skills, AgentDefault::Fixed(vec![]));
    assert_eq!(restored.mcp, AgentDefault::Fixed(vec![]));
}

#[test]
fn agent_defaults_form_rejects_incompatible_engine_values() {
    let mut defaults = AgentDefaults::default();
    defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"allow"}));
    assert!(
        super::defaults::validate(
            &AgentRuntime::Acp { backend: "codex".into() },
            &defaults,
            &[],
            &[],
            &[]
        )
        .is_err()
    );
    defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"deny"}));
    assert!(
        super::defaults::validate(
            &AgentRuntime::Acp { backend: "codex".into() },
            &defaults,
            &[],
            &[],
            &[]
        )
        .is_ok()
    );
    defaults.mcp = AgentDefault::Fixed(vec!["disabled".into()]);
    assert!(super::defaults::validate(&AgentRuntime::Native, &defaults, &[], &[], &[]).is_err());
}

#[test]
fn agent_defaults_form_inherit_patch_removes_fixed_payloads() {
    let patch = super::defaults::patch(&AgentDefaults::default());
    for field in ["model", "permission", "skills", "mcp"] {
        assert_eq!(patch[field]["mode"], "inherit");
        assert!(patch[field].as_object().unwrap().contains_key("value"));
        assert!(patch[field]["value"].is_null());
    }
}

#[test]
fn agent_defaults_form_switch_preserves_invalid_draft_and_prevents_save() {
    use crate::app::{
        App,
        message::settings::{AgentsMessage, SettingsMessage},
    };
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into()];
    let dispatch = |app: &mut App, message| {
        let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
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
    use vw_config_types::agent::AgentModelReference;
    let app = crate::app::App::new().0;
    let mut draft = DelegateAgentSettingsEntry::from_config("custom", None);
    draft.force_model_selection = true;
    draft.provider = "provider".into();
    draft.model = "forced".into();
    draft.defaults.model = AgentDefault::Fixed(AgentModelReference {
        provider: "provider".into(),
        model: "other".into(),
    });
    assert!(super::defaults::validation_error(&app, &draft).unwrap().contains("强制模型绑定冲突"));
}

#[test]
fn agent_config_cleanup_single_model_group_prevents_conflict() {
    use crate::app::{
        App,
        message::settings::{AgentsMessage, SettingsMessage},
    };
    use vw_config_types::agent::AgentModelReference;
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    let dispatch = |app: &mut App, message| {
        let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
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
    let app = crate::app::App::new().0;
    let mut draft = app.agents_settings.entries[0].clone();
    draft.skill_policy = vw_config_types::agent::DelegateCapabilityPolicy::Strict;
    draft.allowed_skills = vec![];
    draft.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    assert!(super::defaults::validation_error(&app, &draft).is_some());
    draft.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(super::defaults::validation_error(&app, &draft).is_none());
    let empty = super::defaults::patch(&draft.defaults);
    assert_eq!(empty["skills"]["value"], serde_json::json!([]));
    draft.defaults.skills = AgentDefault::Inherit;
    assert_eq!(super::defaults::patch(&draft.defaults)["skills"]["value"], serde_json::Value::Null);
}

#[test]
fn agent_config_cleanup_full_access_patch_preserves_permission() {
    let defaults = AgentDefaults {
        full_access: true,
        permission: AgentDefault::Fixed(serde_json::json!({"*":"deny"})),
        ..Default::default()
    };
    let patch = super::defaults::patch(&defaults);
    assert_eq!(patch["full_access"], true);
    assert_eq!(patch["permission"]["value"], serde_json::json!({"*":"deny"}));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_model_inheritance_renders_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = crate::app::App::new().0;
        app.agents_settings.config_ready = true;
        let _ = crate::app::message::settings::update(
            &mut app,
            crate::app::message::settings::SettingsMessage::Agents(
                crate::app::message::settings::AgentsMessage::ProfileOpen("main".into()),
            ),
        );

        app.agents_settings.profile_modal.as_mut().unwrap().runtime =
            AgentRuntime::Acp { backend: "codex".into() };
        app.mcp_settings.inspect_loading = false;
        app.mcp_settings.inspect_error = Some("隔离目录加载失败".into());
        app.agents_settings.profile_modal.as_mut().unwrap().defaults.mcp =
            AgentDefault::Fixed(vec!["中文服务".into()]);
        let mut element: iced::Element<'_, crate::app::Message> = iced::widget::container(
            super::defaults::view(&app, app.agents_settings.profile_modal.as_ref().unwrap()),
        )
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

#[test]
fn agent_model_inheritance_verified_acp_models_survive_disconnect_and_reopen() {
    use crate::app::state::{ModelSummary, ProviderModelsSummary, ProviderSummary};
    use vw_config_types::agent::AgentModelReference;
    use vw_gateway_client::vw_api_types::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability,
    };
    let mut app = crate::app::App::new().0;
    let model = AgentModelReference { provider: "provider".into(), model: "selected".into() };
    app.agents_settings.providers = vec![ProviderSummary {
        id: "provider".into(),
        name: "Provider".into(),
        source_label: crate::app::i18n::settings_models::messages::Feedback::owned("配置", vec![]),
        connected: true,
    }];
    app.agents_settings.provider_models = vec![ProviderModelsSummary {
        id: "provider".into(),
        name: "Provider".into(),
        models: vec![ModelSummary {
            id: "selected".into(),
            name: "Selected".into(),
            enabled: true,
            toolcall: true,
            attachment: false,
            context_limit: 8192,
            detail: serde_json::Value::Null,
        }],
    }];
    let runtime = app.get_session_runtime_mut("verified");
    runtime.acp_agent = Some("codex".into());
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::ChatMessage::AcpSourceLoaded {
            session_id: "verified".into(),
            agent: "codex".into(),
            result: Ok(vw_gateway_client::vw_api_types::acp_session::AcpSessionOptions {
                connected: false,
                model_source_capabilities: vec![AcpModelSourceCapability::Supported {
                    source: AcpModelSource::SessionModel,
                }],
                ..Default::default()
            }),
        },
    );
    let config = AgentDefinitionConfig {
        runtime: AgentRuntime::Acp { backend: "codex".into() },
        defaults: AgentDefaults { model: AgentDefault::Fixed(model), ..Default::default() },
        ..Default::default()
    };
    for draft in [
        {
            let mut draft = DelegateAgentSettingsEntry::from_config("custom", None);
            draft.runtime = config.runtime.clone();
            draft.defaults = config.defaults.clone();
            draft
        },
        DelegateAgentSettingsEntry::from_config("custom", Some(config)),
    ] {
        assert_eq!(super::defaults::validation_error(&app, &draft), None);
        app.session_runtime_states.get_mut("verified").unwrap().acp_agent = Some("other".into());
        assert!(super::defaults::validation_error(&app, &draft).is_some());
        app.session_runtime_states.get_mut("verified").unwrap().acp_agent = Some("codex".into());
    }
    app.agents_settings.provider_models[0].models[0].enabled = false;
    let draft = DelegateAgentSettingsEntry::from_config(
        "custom",
        Some(AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "codex".into() },
            defaults: AgentDefaults {
                model: AgentDefault::Fixed(AgentModelReference {
                    provider: "provider".into(),
                    model: "selected".into(),
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    assert!(super::defaults::validation_error(&app, &draft).is_some());
    app.agents_settings.provider_models[0].models[0].enabled = true;
    app.agents_settings.providers[0].connected = false;
    assert!(super::defaults::validation_error(&app, &draft).is_some());
}
