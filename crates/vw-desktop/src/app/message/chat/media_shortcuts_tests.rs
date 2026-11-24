use super::*;
use iced::widget::text_editor;

#[test]
fn media_shortcuts_selection_and_exit_preserve_the_session_draft() {
    let mut app = App::new().0;
    app.session_state.active_session_id = Some("media-a".into());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("原有草稿");
    app.session_state.files = vec!["/tmp/original.png".into()];

    for mode in [
        MediaMode::Image,
        MediaMode::Video,
        MediaMode::Speech,
        MediaMode::Vision,
        MediaMode::Transcription,
    ] {
        let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(Some(mode))));
        assert_eq!(active_mode(&app), Some(mode));
        assert!(!app.current_session_runtime().request_lifecycle.is_requesting);
    }

    app.session_state.active_session_id = Some("media-b".into());
    assert_eq!(active_mode(&app), None);
    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(Some(MediaMode::Speech))));
    app.session_state.active_session_id = Some("media-a".into());
    assert_eq!(active_mode(&app), Some(MediaMode::Transcription));

    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(None)));
    assert_eq!(active_mode(&app), None);
    assert_eq!(app.current_session_runtime().input_editor.text().trim_end(), "原有草稿");
    assert_eq!(app.session_state.files, ["/tmp/original.png"]);
}

#[test]
fn media_shortcuts_picker_ignores_results_for_a_different_session() {
    let mut app = App::new().0;
    app.session_state.active_session_id = Some("media-b".into());
    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(Some(MediaMode::Vision))));
    let project_path = app.navigation_tabs.project_path.clone();
    let _ = app.update(Message::Chat(ChatMessage::MediaAttachmentsPicked {
        session_id: Some("media-a".into()),
        project_path,
        mode: MediaMode::Vision,
        paths: vec!["/missing/clip.mp4".into()],
    }));
    assert!(app.session_state.files.is_empty());
    assert!(app.notification_state.notifications.is_empty());
}

#[test]
fn media_shortcuts_preserve_editor_content_before_the_runtime_exists() {
    let mut app = App::new().0;
    app.session_state.active_session_id = Some("media-uninitialized".into());
    app.session_state.input_editor = text_editor::Content::with_text("仍在输入");
    assert!(app.current_session_runtime_ref().is_none());

    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(Some(MediaMode::Image))));
    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(None)));

    assert_eq!(app.current_session_runtime().input_editor.text().trim_end(), "仍在输入");
}

#[test]
fn media_shortcuts_drop_filters_attachments_without_editing_the_draft() {
    let mut app = App::new().0;
    let folder = tempfile::tempdir().unwrap();
    let clip = folder.path().join("clip.mp4");
    let document = folder.path().join("notes.txt");
    std::fs::write(&clip, "fixture video").unwrap();
    std::fs::write(&document, "fixture text").unwrap();
    app.navigation_tabs.project_path = Some(folder.path().to_string_lossy().into_owned());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("分析视频");
    let _ = app.update(Message::Chat(ChatMessage::MediaModeSelected(Some(MediaMode::Vision))));
    app.file_tree_interaction.dragging_file_paths = vec!["clip.mp4".into(), "notes.txt".into()];

    let _ = app.update(Message::Chat(ChatMessage::InputAreaDragDrop));

    assert_eq!(
        app.session_state.files,
        [clip.canonicalize().unwrap().to_string_lossy().into_owned()]
    );
    assert!(app.file_tree_interaction.dragging_file_paths.is_empty());
    assert_eq!(app.current_session_runtime().input_editor.text().trim_end(), "分析视频");
}

#[test]
fn media_shortcuts_video_edit_dispatches_the_existing_template() {
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some("/project".into());
    app.session_state.active_session_id = Some("media-a".into());
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("保留草稿");

    let _ = app.update(Message::Chat(ChatMessage::OpenMediaVideoEditor));

    assert_eq!(app.navigation_tabs.screen, crate::app::Screen::WorkflowTool);
    assert!(app.workflow_state.video_edit.open);
    assert!(!app.current_session_runtime().request_lifecycle.is_requesting);
    assert_eq!(app.current_session_runtime().input_editor.text().trim_end(), "保留草稿");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn media_shortcuts_studio_dispatches_to_registered_installation() {
    use crate::app::account::{
        AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
    };
    use desktop_app_launch::{ShijuAppLaunchState, ShijuAppsLoadState};

    let mut app = App::new().0;
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("account-secret").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);

    let _ = app.update(Message::Chat(ChatMessage::OpenMediaAppStudio));
    let ShijuAppsLoadState::Loading { generation } = app.shiju_apps.mine else {
        panic!("expected the installed apps lookup");
    };
    let installed = serde_json::from_value(serde_json::json!({
        "apps": [{
            "installationId": "STUDIO01",
            "appId": "app-studio",
            "name": "Studio",
            "version": "1",
            "source": "official"
        }]
    }))
    .unwrap();
    let account_id = app.account.active_account().unwrap().id().to_owned();
    let _ = app.update(Message::ShijuApps(crate::app::message::ShijuAppsMessage::MineLoaded {
        generation,
        account_id,
        result: Ok(installed),
    }));

    assert_eq!(app.navigation_tabs.screen, crate::app::Screen::ShijuApps);
    assert!(matches!(
        app.shiju_apps.launch,
        ShijuAppLaunchState::CheckingBinding { ref installation_id, .. }
            if installation_id == "STUDIO01"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn media_shortcuts_studio_without_account_shows_configuration_hint() {
    let mut app = App::new().0;
    app.account.replace_repository(crate::app::account::AccountRepository::default());
    let _ = app.update(Message::Chat(ChatMessage::OpenMediaAppStudio));
    assert!(app.toast_state.active_toast.is_some());
}
