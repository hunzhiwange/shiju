use crate::app::{
    App,
    message::settings::{AgentsMessage, SettingsMessage},
};

fn dispatch(app: &mut App, message: AgentsMessage) {
    let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
}

#[test]
fn agent_profile_modal_cancel_preserves_saved_profile() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].label.clone();
    let rules = app.agents_settings.entries[0].system_prompt_editor.text();
    let key = app.agents_settings.entries[0].key.clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("中文资料：长文本".repeat(30)));
    dispatch(
        &mut app,
        AgentsMessage::SystemPromptAction(
            "main".into(),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("取消的规则".to_string()),
            )),
        ),
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.entries[0].label, original);
    assert_eq!(app.agents_settings.entries[0].system_prompt_editor.text(), rules);
    assert!(app.agents_settings.profile_modal.is_none());
}

#[test]
fn agent_profile_modal_failed_save_preserves_draft() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].label.clone();
    let key = app.agents_settings.entries[0].key.clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("待重试的资料".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("受控保存失败".to_string())));
    assert_eq!(app.agents_settings.entries[0].label, original);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, "待重试的资料");
}

#[test]
fn agent_profile_modal_builtin_rejects_engine_switch_and_delete() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileDelete);
    assert!(!app.agents_settings.profile_saving);
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        vw_config_types::agent::AgentRuntime::Native
    );
}

#[test]
fn agent_profile_modal_prompts_preserve_user_order() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptAdd);
    dispatch(&mut app, AgentsMessage::ProfilePromptChanged(0, "第一项".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptAdd);
    dispatch(&mut app, AgentsMessage::ProfilePromptChanged(1, "第二项".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptMove(1, 0));
    dispatch(&mut app, AgentsMessage::ProfilePromptRemove(1));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().recommended_prompts,
        vec!["第二项"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_config_cleanup_five_tabs_render_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name, tab) in [
        (Theme::Light, "light", crate::app::state::AGENT_DETAIL_BASIC_TAB),
        (Theme::Dark, "dark", crate::app::state::AGENT_DETAIL_BASIC_TAB),
        (Theme::Light, "light", crate::app::state::AGENT_DETAIL_IDENTITY_TAB),
        (Theme::Dark, "dark", crate::app::state::AGENT_DETAIL_IDENTITY_TAB),
        (Theme::Light, "light", crate::app::state::AGENT_DETAIL_TOOLS_TAB),
        (Theme::Dark, "dark", crate::app::state::AGENT_DETAIL_TOOLS_TAB),
        (Theme::Light, "light", crate::app::state::AGENT_DETAIL_SKILLS_TAB),
        (Theme::Dark, "dark", crate::app::state::AGENT_DETAIL_SKILLS_TAB),
        (Theme::Light, "light", crate::app::state::AGENT_DETAIL_MCP_TAB),
        (Theme::Dark, "dark", crate::app::state::AGENT_DETAIL_MCP_TAB),
    ] {
        for (width, height) in [(1100, 900), (760, 520)] {
            let mut app = App::new().0;
            app.agents_settings.config_ready = true;
            dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
            dispatch(
                &mut app,
                AgentsMessage::ProfileNameChanged("中文智能体资料与 English".repeat(8)),
            );
            dispatch(
                &mut app,
                AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::Edit(
                    iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                        "中文长描述，用于验证字形整形和暗黑主题。\n多行描述。\n".repeat(10),
                    )),
                )),
            );
            dispatch(
                &mut app,
                AgentsMessage::SystemPromptAction(
                    "main".into(),
                    iced::widget::text_editor::Action::Edit(
                        iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                            "规则：审查代码并解释原因。\n".repeat(20),
                        )),
                    ),
                ),
            );
            app.agents_settings.active_detail_tab = tab.to_string();
            let mut element = super::profile_modal::overlay(
                &app,
                iced::widget::container(iced::widget::text("背景"))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .into(),
            );
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let bounds = Size::new(width as f32, height as f32);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            #[derive(Default)]
            struct Labels(Vec<String>, usize, Vec<(String, Rectangle)>);
            impl iced::advanced::widget::Operation for Labels {
                fn traverse(
                    &mut self,
                    operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
                ) {
                    operate(self);
                }
                fn focusable(
                    &mut self,
                    _id: Option<&iced::advanced::widget::Id>,
                    _bounds: Rectangle,
                    _state: &mut dyn iced::advanced::widget::operation::Focusable,
                ) {
                    self.1 += 1;
                }
                fn text(
                    &mut self,
                    _id: Option<&iced::advanced::widget::Id>,
                    bounds: Rectangle,
                    text: &str,
                ) {
                    self.0.push(text.to_owned());
                    self.2.push((text.to_owned(), bounds));
                }
            }
            let mut labels = Labels::default();
            element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
            assert!(!labels.0.iter().any(|text| text == "资料与默认参数" || text == "规则"));
            for label in ["基础参数", "身份", "工具", "技能", "MCP", "取消", "保存配置"]
            {
                assert!(labels.0.iter().any(|text| text == label), "missing {label} on {tab}");
            }
            for label in ["智能体配置", "取消", "保存配置"] {
                let bounds = labels.2.iter().find(|(text, _)| text == label).unwrap().1;
                assert!(
                    bounds.y >= 0.0 && bounds.y + bounds.height <= height as f32,
                    "fixed header/footer clipped: {label} {bounds:?}"
                );
            }
            let count = |label: &str| labels.0.iter().filter(|text| text.as_str() == label).count();
            if tab == crate::app::state::AGENT_DETAIL_BASIC_TAB {
                assert_eq!(count("名称"), 1);
                assert_eq!(count("描述"), 1);
                assert_eq!(count("模型"), 1);
                assert_eq!(count("系统提示词"), 0);
            } else if tab == crate::app::state::AGENT_DETAIL_IDENTITY_TAB {
                assert_eq!(count("系统提示词"), 2); // tab and field label
                assert_eq!(labels.1, 1); // a single editable system prompt
                assert_eq!(count("描述"), 0);
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(width, height), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
            image::save_buffer(
                format!("/tmp/shiju-task7-{name}-{tab}-{width}x{height}.png"),
                &pixels,
                width,
                height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[test]
fn agent_config_cancel_discards_parameters_and_profile_together() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(original.key.clone()));
    dispatch(&mut app, AgentsMessage::TemperatureChanged(original.key.clone(), 1.5));
    dispatch(&mut app, AgentsMessage::MaxIterationsChanged(original.key.clone(), 123));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("修改名称".into()));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().max_iterations, 123);
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.entries[0].temperature, original.temperature);
    assert_eq!(app.agents_settings.entries[0].max_iterations, original.max_iterations);
    assert_eq!(app.agents_settings.entries[0].label, original.label);
    assert!(!app.agents_settings.dirty);
}

#[test]
fn agent_config_create_commits_only_after_successful_save() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("custom-agent".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert!(!app.agents_settings.entries.iter().any(|e| e.key == "custom-agent"));
    dispatch(&mut app, AgentsMessage::MaxIterationsChanged("custom-agent".into(), 123));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("新智能体".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let entry = app.agents_settings.entries.iter().find(|e| e.key == "custom-agent").unwrap();
    assert_eq!(entry.label, "新智能体");
    assert_eq!(entry.max_iterations, 123);
    assert!(app.agents_settings.profile_modal.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_skill_directory_renders_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = App::new().0;
        app.agents_settings.config_ready = true;
        app.project_path = Some("/应用项目".into());
        app.recent_projects = vec!["/技能目录甲".into(), "/技能目录乙".into()];
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
        dispatch(&mut app, AgentsMessage::SkillDirectorySelected(Some("/技能目录乙".into())));
        app.agents_settings.active_detail_tab = crate::app::state::AGENT_DETAIL_SKILLS_TAB.into();
        let mut element = super::profile_modal::overlay(
            &app,
            iced::widget::container(iced::widget::text("背景"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
        );
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
            format!("/tmp/shiju-task2-skill-directory-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

#[test]
fn agent_config_cleanup_basic_combines_profile_and_runtime() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
    assert_eq!(app.agents_settings.active_detail_tab, crate::app::state::AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("合并配置".into()));
    dispatch(&mut app, AgentsMessage::TemperatureChanged("main".into(), 1.3));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let entry = app.agents_settings.entries.iter().find(|e| e.key == "main").unwrap();
    assert_eq!(entry.label, "合并配置");
    assert_eq!(entry.temperature, 1.3);
}

#[test]
fn agent_config_cleanup_single_system_prompt_preserves_content() {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    let key = app.agents_settings.entries[0].key.clone();
    let before = app.agents_settings.entries[0].system_prompt_editor.text();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key.clone()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().system_prompt_editor.text(),
        before
    );
    dispatch(
        &mut app,
        AgentsMessage::SystemPromptAction(
            key,
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("身份规则\n中文".into()),
            )),
        ),
    );
    dispatch(&mut app, AgentsMessage::ProfileSave);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    assert!(app.agents_settings.entries[0].system_prompt_editor.text().contains("身份规则\n中文"));
}

#[test]
fn agent_config_cleanup_multiline_description_roundtrips_and_cancels() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("multiline".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert_eq!(app.agents_settings.active_detail_tab, crate::app::state::AGENT_DETAIL_BASIC_TAB);
    let description = "中文第一行\n第二行 English\n第三行";
    dispatch(&mut app, AgentsMessage::ProfileDescriptionAction(Action::SelectAll));
    dispatch(
        &mut app,
        AgentsMessage::ProfileDescriptionAction(Action::Edit(Edit::Paste(std::sync::Arc::new(
            description.into(),
        )))),
    );
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("隔离失败".into())));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().description, description);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    dispatch(&mut app, AgentsMessage::ProfileOpen("multiline".into()));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert_eq!(draft.description, description);
    assert_eq!(draft.description_editor.text(), description);
    dispatch(
        &mut app,
        AgentsMessage::ProfileDescriptionAction(Action::Edit(Edit::Paste(std::sync::Arc::new(
            "取消内容".into(),
        )))),
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    dispatch(&mut app, AgentsMessage::ProfileOpen("multiline".into()));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().description, description);
}

fn runtime_creation_draft() -> App {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("runtime-custom".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    app
}

#[test]
fn agent_runtime_creation_success_locks_runtime_but_keeps_profile_editable() {
    use vw_config_types::agent::{AgentDefault, AgentRuntime};
    let mut app = runtime_creation_draft();
    for backend in ["codex", "claude", "native", "codex"] {
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
        let expected = if backend == "native" {
            AgentRuntime::Native
        } else {
            AgentRuntime::Acp { backend: backend.into() }
        };
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("unknown-custom-name".into()));
    let expected = AgentRuntime::Acp { backend: "codex".into() };
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    dispatch(&mut app, AgentsMessage::ProfileOpen("runtime-custom".into()));
    for backend in ["native", "claude"] {
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("保存后仍可改名".into()));
    dispatch(&mut app, AgentsMessage::ProfileModelChanged(AgentDefault::Inherit));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert_eq!(draft.label, "保存后仍可改名");
    assert_eq!(draft.defaults.model, AgentDefault::Inherit);
}

#[test]
fn agent_runtime_creation_failed_save_allows_retry_and_cancel_leaves_no_entry() {
    use vw_config_types::agent::AgentRuntime;
    let mut app = runtime_creation_draft();
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("受控保存失败".into())));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        AgentRuntime::Acp { backend: "claude".into() }
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert!(app.agents_settings.profile_modal.is_none());
    assert!(!app.agents_settings.entries.iter().any(|e| e.key == "runtime-custom"));
}

#[test]
fn agent_runtime_creation_reloaded_custom_and_builtin_reject_changes() {
    use vw_config_types::agent::{AgentRuntime, DelegateAgentConfig};
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    let config = DelegateAgentConfig {
        runtime: AgentRuntime::Acp { backend: "claude".into() },
        ..Default::default()
    };
    app.agents_settings
        .entries
        .push(crate::app::state::DelegateAgentSettingsEntry::from_config("reloaded", Some(config)));
    for key in ["reloaded", "main"] {
        dispatch(&mut app, AgentsMessage::ProfileOpen(key.into()));
        let expected = app.agents_settings.profile_modal.as_ref().unwrap().runtime.clone();
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
}

#[test]
fn agent_runtime_creation_empty_directory_refreshes_and_failure_preserves_native() {
    use crate::app::message::settings::AcpMessage;
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents.clear();
    app.acp_settings.loading = false;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("empty-catalog".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert!(app.acp_settings.loading);
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::Loaded(Err("目录加载失败".into()))),
    );
    assert!(!app.acp_settings.loading);
    assert!(app.acp_settings.save_error.is_some());
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        vw_config_types::agent::AgentRuntime::Native
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_runtime_creation_controls_render_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for state in ["new", "loading", "failed", "saved"] {
            let mut app = runtime_creation_draft();
            if state == "saved" {
                dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
                dispatch(&mut app, AgentsMessage::ProfileSave);
                dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
                dispatch(&mut app, AgentsMessage::ProfileOpen("runtime-custom".into()));
            } else if state != "new" {
                app.acp_agents.clear();
                app.acp_settings.loading = state == "loading";
                if state == "failed" {
                    app.acp_settings.save_error = Some("目录加载失败，请重试".into());
                }
            }
            let draft = app.agents_settings.profile_modal.as_ref().unwrap();
            let mut element = super::basic_profile::view(&app, draft);
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let bounds = Size::new(920.0, 760.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
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
                    value: &str,
                ) {
                    self.0.push(value.into());
                }
            }
            let mut labels = Labels::default();
            element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
            assert!(labels.0.iter().any(|s| s.contains("首次保存成功后")));
            if state == "saved" {
                assert!(labels.0.iter().any(|s| s == "固定运行引擎：codex"));
                assert!(!labels.0.iter().any(|s| s == "刷新引擎目录"));
            } else if state == "loading" {
                assert!(labels.0.iter().any(|s| s == "正在加载 ACP 引擎目录…"));
            } else if state == "failed" {
                assert!(labels.0.iter().any(|s| s == "目录加载失败，请重试"));
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(920, 760), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
            image::save_buffer(
                format!("/tmp/shiju-runtime-creation-{name}-{state}.png"),
                &pixels,
                920,
                760,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[test]
fn agent_runtime_visibility_switch_closes_and_rejects_tools_tab_without_clearing_values() {
    use crate::app::state::{AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_TOOLS_TAB};
    let mut app = runtime_creation_draft();
    let draft = app.agents_settings.profile_modal.as_mut().unwrap();
    draft.api_key_input = "retained-override".into();
    draft.temperature = 1.23;
    draft.allowed_tools = vec!["shell".into()];
    draft.max_iterations = 123;
    draft.max_depth = 7;
    draft.max_parallel_subagents = 3;
    draft.compact_context = true;
    draft.parallel_tools = true;
    draft.max_history_messages = 42;
    let original = draft.clone();
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_TOOLS_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("native".into()));
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_TOOLS_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let saved = app.agents_settings.entries.iter().find(|e| e.key == original.key).unwrap();
    assert_eq!(saved.allowed_tools, original.allowed_tools);
    assert_eq!(saved.max_iterations, original.max_iterations);
    assert_eq!(saved.max_depth, original.max_depth);
    assert_eq!(saved.max_parallel_subagents, original.max_parallel_subagents);
    assert_eq!(saved.api_key_input, original.api_key_input);
    assert_eq!(saved.temperature, original.temperature);
    assert_eq!(saved.defaults.permission, original.defaults.permission);
    assert_eq!(saved.compact_context, original.compact_context);
    assert_eq!(saved.parallel_tools, original.parallel_tools);
    assert_eq!(saved.max_history_messages, original.max_history_messages);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_runtime_visibility_actual_modal_renders_native_and_acp_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for backend in ["native", "codex", "claude"] {
        for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            let mut app = runtime_creation_draft();
            dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
            for tab in ["basic", "identity", "skills", "mcp", "tools"] {
                // Exercise the view's defensive fallback as well as reducer validation.
                app.agents_settings.active_detail_tab = tab.into();
                let mut element = super::profile_modal::overlay(
                    &app,
                    iced::widget::container(iced::widget::text("背景"))
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .into(),
                );
                let mut renderer = <iced::Renderer as Headless>::new(
                    Font::DEFAULT,
                    Pixels(16.0),
                    Some("tiny-skia"),
                )
                .await
                .unwrap();
                let bounds = Size::new(1100.0, 900.0);
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, bounds),
                );
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
                        _: Option<&iced::advanced::widget::Id>,
                        _: Rectangle,
                        value: &str,
                    ) {
                        self.0.push(value.into());
                    }
                }
                let mut labels = Labels::default();
                element.as_widget_mut().operate(
                    &mut tree,
                    Layout::new(&node),
                    &renderer,
                    &mut labels,
                );
                let has = |label: &str| labels.0.iter().any(|s| s == label);
                for label in ["基础参数", "身份", "技能", "MCP", "保存配置"] {
                    assert!(has(label), "{backend}/{tab}: {label}");
                }
                assert_eq!(has("工具"), backend == "native");
                if tab == "skills" {
                    assert!(has("技能目录"));
                }
                if tab == "mcp" {
                    assert!(has("默认 MCP"));
                }
                if tab == "identity" {
                    assert!(has("系统提示词"));
                }
                if tab == "basic" || (tab == "tools" && backend != "native") {
                    for label in ["名称", "描述", "模型", "权限", "作为子智能体"] {
                        assert!(has(label), "{backend}: {label}");
                    }
                    for label in [
                        "API 密钥",
                        "温度",
                        "最大深度",
                        "子智能体数量",
                        "最大迭代次数",
                        "智能体模式",
                        "上下文压缩",
                        "历史消息上限",
                        "并行工具",
                    ] {
                        assert_eq!(has(label), backend == "native", "{backend}: {label}");
                    }
                }
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
                assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
                if tab == "basic" {
                    image::save_buffer(
                        format!("/tmp/shiju-runtime-visibility-{backend}-{name}.png"),
                        &pixels,
                        1100,
                        900,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
        }
    }
}
