use super::*;
use iced::widget::text_editor;

#[test]
fn media_shortcut_appends_to_the_chat_draft_without_sending() {
    let mut app = App::new().0;
    app.session_state.session_runtime_states.clear();
    app.session_state.input_editor = text_editor::Content::with_text("原有草稿");
    app.session_state.files = vec!["/tmp/reference.png".into()];
    app.session_state.input_popovers.set_media_shortcuts(true);

    let _ =
        app.update(Message::Chat(ChatMessage::MediaPromptSelected("帮我生成一张图片：".into())));

    assert!(!app.session_state.input_popovers.media_shortcuts);
    assert!(!app.current_session_runtime().request_lifecycle.is_requesting);
    assert_eq!(app.session_state.input_editor.text().trim_end(), "原有草稿\n帮我生成一张图片：",);
    assert_eq!(app.session_state.files, ["/tmp/reference.png"]);
}

#[test]
fn media_shortcuts_select_their_installed_skills_without_toggling_them_off() {
    use crate::app::skill_hub::LoadState;

    for (message, slug, name) in [
        (ChatMessage::SelectVideoEditorSkill, "cutvideo", "识矩剪辑"),
        (ChatMessage::SelectAppDevelopmentSkill, "shiju-app", "识矩应用"),
        (ChatMessage::SelectAppManagementSkill, "shiju-app-admin", "识矩应用管理"),
    ] {
        let mut app = App::new().0;
        app.session_state.active_session_id = Some("chat-a".into());
        app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("保留草稿");
        app.expert_hub.installed = LoadState::Loaded(
            serde_json::from_value(serde_json::json!([{
                "skill_id": 712,
                "slug": slug,
                "name": name,
                "description": "",
                "installed_version": "2.1",
                "current_version": "2.1",
                "format": "markdown",
                "upgrade_available": false
            }]))
            .unwrap(),
        );
        let screen = app.navigation_tabs.screen;

        for _ in 0..2 {
            app.session_state.input_popovers.set_media_shortcuts(true);
            let _ = app.update(Message::Chat(message.clone()));

            assert!(!app.session_state.input_popovers.media_shortcuts);
            assert_eq!(app.navigation_tabs.screen, screen);
            assert!(!app.workflow_state.video_edit.open);
            let runtime = app.current_session_runtime();
            assert!(runtime.tool_selector.is_shiju_skill_selected(712), "{slug}");
            assert_eq!(runtime.input_editor.text().trim_end(), "保留草稿");
        }
    }
}
