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
        AgentsMessage::ProfileRulesAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Paste(std::sync::Arc::new("取消的规则".to_string())),
        )),
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
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = App::new().0;
        app.agents_settings.config_ready = true;
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
        dispatch(&mut app, AgentsMessage::ProfileNameChanged("中文智能体资料与 English".repeat(8)));
        dispatch(
            &mut app,
            AgentsMessage::ProfileDescriptionChanged(
                "中文长描述，用于验证字形整形和暗黑主题。".repeat(10),
            ),
        );
        dispatch(
            &mut app,
            AgentsMessage::ProfileRulesAction(iced::widget::text_editor::Action::Edit(
                iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                    "规则：审查代码并解释原因。\n".repeat(20),
                )),
            )),
        );
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
