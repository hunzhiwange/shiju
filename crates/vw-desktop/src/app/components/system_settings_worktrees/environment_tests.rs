use crate::app::message::settings::worktrees::environment::{EnvironmentMessage as Event, apply};
fn project(id: &str) -> vw_gateway_client::vw_api_types::project::ProjectDto {
    serde_json::from_value(serde_json::json!({
        "id":id,"name":format!("中文项目-{id}"),"directory":format!("/项目/{id}"),
        "display_path":format!("/项目/{id}"),"status":"ready","created_at_ms":1,"updated_at_ms":2,
        "git":{"is_repo":true,"has_uncommitted_changes":false}
    }))
    .unwrap()
}
#[test]
fn worktree_environment_ui_saves_setup_for_selected_project() {
    let mut app = crate::app::App::new().0;
    app.project_id = Some("a".into());
    apply(&mut app, Event::Edit(project("b")));
    loaded(&mut app, Some("original"));
    app.worktree_settings.environment.editor =
        iced::widget::text_editor::Content::with_text("echo 环境\ncargo build");
    let (selected, patch) = apply(&mut app, Event::Save).expect("selected project must be saved");
    assert_eq!(selected.id.as_ref(), "b");
    assert_eq!(patch.commands.unwrap().start.as_deref(), Some("echo 环境\ncargo build"));
    assert_eq!(app.project_id.as_deref(), Some("a"));
}
#[test]
fn worktree_environment_ui_cancel_preserves_command() {
    let mut app = crate::app::App::new().0;
    apply(&mut app, Event::Edit(project("a")));
    loaded(&mut app, Some("original"));
    app.worktree_settings.environment.editor =
        iced::widget::text_editor::Content::with_text("unsaved");
    assert!(apply(&mut app, Event::Cancel).is_none());
    assert!(app.worktree_settings.environment.selected.is_none());
    assert!(!app.worktree_settings.environment.saving);
}
#[test]
fn worktree_environment_ui_save_does_not_execute() {
    let mut app = crate::app::App::new().0;
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("must-not-exist");
    apply(&mut app, Event::Edit(project("a")));
    loaded(&mut app, Some("original"));
    app.worktree_settings.environment.editor =
        iced::widget::text_editor::Content::with_text(&format!("touch '{}'", sentinel.display()));
    assert!(apply(&mut app, Event::Save).is_some());
    assert!(!sentinel.exists());
    assert!(!app.new_session_creating);
}

fn loaded(app: &mut crate::app::App, command: Option<&str>) {
    let identity = crate::app::state::WorktreeSettingsIdentity::current(app);
    let generation = app.worktree_settings.environment.generation;
    apply(app, Event::Loaded { identity, generation, result: Ok(command.map(str::to_owned)) });
}
#[test]
fn worktree_environment_ui_clear_failure_retry_and_stale_response() {
    let mut app = crate::app::App::new().0;
    apply(&mut app, Event::Edit(project("a")));
    let old = app.worktree_settings.environment.generation;
    loaded(&mut app, Some("old"));
    apply(&mut app, Event::Edit(project("b")));
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
    apply(
        &mut app,
        Event::Loaded {
            identity: identity.clone(),
            generation: old,
            result: Ok(Some("wrong".into())),
        },
    );
    assert!(!app.worktree_settings.environment.loaded);
    loaded(&mut app, Some("b original"));
    app.worktree_settings.environment.editor = iced::widget::text_editor::Content::with_text("");
    let (p, patch) = apply(&mut app, Event::Save).unwrap();
    assert_eq!(p.id.as_ref(), "b");
    assert!(patch.commands.unwrap().start.is_none());
    let generation = app.worktree_settings.environment.generation;
    apply(
        &mut app,
        Event::Saved {
            identity: identity.clone(),
            generation,
            result: Err("network failed".into()),
        },
    );
    assert_eq!(app.worktree_settings.environment.editor.text(), "");
    assert!(app.worktree_settings.environment.selected.is_some());
    assert_eq!(app.worktree_settings.environment.error.as_deref(), Some("network failed"));
    assert!(apply(&mut app, Event::Save).is_some());
    let generation = app.worktree_settings.environment.generation;
    apply(&mut app, Event::Saved { identity, generation, result: Ok(()) });
    assert!(app.worktree_settings.environment.selected.is_none());
}
#[test]
fn worktree_environment_ui_rejects_remote_picker_and_deduplicates_additions() {
    use crate::app::message::settings::worktrees::environment::{can_add, update};
    assert!(!can_add(&vw_gateway_client::GatewayEndpoint::new("192.0.2.1", 8080)));
    let mut app = crate::app::App::new().0;
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
    for _ in 0..2 {
        let _ = update(
            &mut app,
            Event::Added { identity: identity.clone(), generation: 0, result: Ok(project("a")) },
        );
    }
    assert_eq!(app.worktree_settings.environment.projects.len(), 1);
    apply(&mut app, Event::Edit(project("a")));
    loaded(&mut app, Some("original"));
    apply(&mut app, Event::Cancel);
    apply(&mut app, Event::Edit(project("a")));
    loaded(&mut app, Some("original"));
    assert_eq!(app.worktree_settings.environment.editor.text(), "original");
}
#[test]
fn worktree_environment_ui_project_switch_discards_pending_save() {
    use crate::app::message::settings::worktrees::environment::update;
    let mut app = crate::app::App::new().0;
    let _ = update(&mut app, Event::Edit(project("a")));
    loaded(&mut app, Some("saved"));
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
    apply(&mut app, Event::Save);
    let generation = app.worktree_settings.environment.generation;
    app.project_path = Some("/project/new-context".into());
    let _ = update(&mut app, Event::Saved { identity, generation, result: Ok(()) });
    assert!(app.worktree_settings.environment.selected.is_none());
    assert!(!app.worktree_settings.environment.saving);
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn worktree_environment_ui_renders_chinese_editor_in_both_themes() {
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Font, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut app = crate::app::App::new().0;
    app.worktree_settings.active_tab = crate::app::state::WorktreesTab::Environment;
    app.worktree_settings.environment.projects = vec![project("中文项目目录")];
    apply(&mut app, Event::Edit(project("中文项目目录")));
    loaded(&mut app, Some("echo 中文初始化命令\ncargo build"));
    app.worktree_settings.environment.error = Some("保存失败，输入已保留。".into());
    for theme in [Theme::Light, Theme::Dark] {
        let base =
            super::page(&app.worktree_settings, crate::app::i18n::I18n::new(app.desktop_locale));
        let element = super::environment::editor(
            &app.worktree_settings.environment,
            crate::app::i18n::I18n::new(app.desktop_locale),
            base,
        );
        let mut ui =
            UserInterface::build(element, Size::new(900.0, 800.0), Cache::new(), &mut renderer);
        ui.update(
            &[],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        assert_eq!(
            renderer.screenshot(Size::new(900, 800), 1.0, theme.palette().background).len(),
            900 * 800 * 4
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn worktree_environment_ui_paginates_and_sends_only_project_patch() {
    use crate::app::message::settings::worktrees::environment::{load_projects, save_command};
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let client = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let sentinel = directory.path().join("must-not-execute");
    let command = format!("touch '{}'", sentinel.display());
    let expected_command = command.clone();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let exchanges = [
            (
                "GET /v1/projects?limit=100 ",
                serde_json::json!({"items":[project("a")],"next_cursor":"next"}),
            ),
            (
                "GET /v1/projects?cursor=next&limit=100 ",
                serde_json::json!({"items":[project("a"),project("b")],"next_cursor":null}),
            ),
            (
                "PATCH /v1/projects/b ",
                serde_json::json!({"project":project("b"),"commands":{"start":expected_command}}),
            ),
        ];
        for (expected, response) in exchanges {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => panic!("missing {expected}: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                header.push(b[0]);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with(expected), "{header}");
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            if expected.starts_with("PATCH") {
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                    serde_json::json!({"commands":{"start":expected_command}})
                );
            }
            let body = response.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    });
    let projects = load_projects(&client, vec![]).await.unwrap();
    assert_eq!(projects.iter().map(|p| p.id.as_ref()).collect::<Vec<_>>(), ["a", "b"]);
    let patch = vw_gateway_client::vw_api_types::project::UpdateProjectRequest {
        commands: Some(vw_gateway_client::vw_api_types::project::CommandsUpdateDto {
            start: Some(command),
        }),
        ..Default::default()
    };
    save_command(&client, &projects[1], &patch).await.unwrap();
    server.join().unwrap();
    assert!(!sentinel.exists());
}

#[test]
fn worktree_environment_ui_retry_blocks_chat_until_refreshed_ready() {
    use crate::app::message::settings::worktrees::{
        ManagedMessage, WorktreeSettingsMessage, update,
    };
    let mut app = crate::app::App::new().0;
    let item = serde_json::from_value(serde_json::json!({
        "worktree":{"id":"failed","project_id":"a","name":"工作树","branch":"feature","directory":"/worktrees/a","status":"error","created_at_ms":1,"updated_at_ms":2},
        "title":"初始化失败","sessions":[],"protection_reasons":[],"recovered_from":null
    })).unwrap();
    app.worktree_settings.managed.groups = vec![crate::app::state::ManagedWorktreeGroup {
        project: project("a"),
        items: vec![item],
        collapsed: false,
        error: None,
    }];
    let event = |e| WorktreeSettingsMessage::Managed(e);
    let _ = update(
        &mut app,
        event(ManagedMessage::Open { project: "a".into(), worktree: "failed".into() }),
    );
    assert!(app.worktree_settings.managed.opening.is_none());
    let _ = update(
        &mut app,
        event(ManagedMessage::Retry { project: "a".into(), worktree: "failed".into() }),
    );
    assert_eq!(app.worktree_settings.managed.retrying, Some(("a".into(), "failed".into())));
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
    let _ = update(
        &mut app,
        event(ManagedMessage::Retried {
            identity,
            target: ("a".into(), "failed".into()),
            result: Ok(()),
        }),
    );
    assert!(app.worktree_settings.managed.retrying.is_none());
    assert!(app.worktree_settings.managed.loading);
    let _ = update(
        &mut app,
        event(ManagedMessage::Open { project: "a".into(), worktree: "failed".into() }),
    );
    assert!(app.worktree_settings.managed.opening.is_none());
}
