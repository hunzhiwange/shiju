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
async fn agent_profile_modal_renders_chinese_long_text_in_both_themes() {
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
    ] {
        let mut app = App::new().0;
        app.agents_settings.config_ready = true;
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
        dispatch(&mut app, AgentsMessage::ProfileNameChanged("中文智能体资料与 English".repeat(8)));
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
                iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                    std::sync::Arc::new("规则：审查代码并解释原因。\n".repeat(20)),
                )),
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
        let bounds = Size::new(1100.0, 900.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        #[derive(Default)]
        struct Labels(Vec<String>, usize);
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
                _bounds: Rectangle,
                text: &str,
            ) {
                self.0.push(text.to_owned());
            }
        }
        let mut labels = Labels::default();
        element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
        assert!(!labels.0.iter().any(|text| text == "资料与默认参数" || text == "规则"));
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
        let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-profile-modal-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
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
