//! 工作区侧栏消息测试，验证折叠宽度、筛选器、分页与空间任务状态。
//! 各标签和弹出层应保持独立，切换视图时不得串用可见数量或悬停状态。

use super::super::{ViewMessage, update as update_panel};
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::WorkSidebarTab;
use crate::app::{App, Screen};

#[test]
fn work_sidebar_toggle_preserves_session_list_width() {
    let mut app = App::new().0;
    app.layout_drag.work_sidebar_width = 360.0;

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(app.navigation_sidebar.work_sidebar_collapsed);
    assert_eq!(app.layout_drag.work_sidebar_width, 360.0);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarMoreOpened);
    assert!(app.navigation_sidebar.work_sidebar_more_open);
    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(!app.navigation_sidebar.work_sidebar_collapsed);
    assert!(!app.navigation_sidebar.work_sidebar_more_open);
    assert_eq!(app.layout_drag.work_sidebar_width, 360.0);
}

#[test]
fn work_sidebar_project_home_leaves_the_session_without_leaving_the_project() {
    let mut app = App::new().0;
    app.navigation_tabs.screen = Screen::Project;
    app.navigation_tabs.project_path = Some("/workspace/project".into());
    app.navigation_tabs.active_tab_id = Some("project_/workspace/project".into());
    app.active_session_id = Some("session-1".into());
    app.navigation_sidebar.work_sidebar_collapsed = true;
    app.navigation_sidebar.work_sidebar_tab = WorkSidebarTab::Spaces;
    app.navigation_workspace.work_resource_half_fullscreen = true;
    app.chat.push(ChatMessage {
        role: ChatRole::User,
        content: "Existing conversation".into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });

    let _task = crate::app::message::view::update(&mut app, ViewMessage::WorkProjectHome);

    assert_eq!(app.navigation_tabs.screen, Screen::Project);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/workspace/project"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("project_/workspace/project"));
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert!(app.session_view_cache.contains("session-1"));
    assert!(app.navigation_sidebar.work_sidebar_collapsed);
    assert!(!app.navigation_workspace.work_resource_half_fullscreen);
    assert_eq!(app.navigation_sidebar.work_sidebar_tab, WorkSidebarTab::Spaces);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn work_sidebar_updates_before_gateway_reply() {
    use crate::app::{Message, config};
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::json;
    use std::sync::mpsc;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(3);

    config::with_editor_settings_gateway(|mut app, requests, responses| {
        app.navigation_sidebar.work_sidebar_collapsed = false;
        let (states, observed_states) = mpsc::channel();
        let (click_again, second_click) = mpsc::channel();
        let gateway = std::thread::spawn(move || {
            let first = requests.recv_timeout(TIMEOUT).expect("first PATCH");
            let collapsed = observed_states.recv_timeout(TIMEOUT);
            click_again.send(()).unwrap();
            let expanded = collapsed.is_ok().then(|| observed_states.recv_timeout(TIMEOUT));
            // Release the response even on failure so a synchronous handler can return.
            responses.send(Some("{}".into())).unwrap();
            assert_eq!(collapsed, Ok(true), "first update must return before the PATCH reply");
            assert_eq!(
                expanded,
                Some(Ok(false)),
                "second update must return before the PATCH reply"
            );

            let second = requests.recv_timeout(TIMEOUT).expect("second PATCH");
            responses.send(Some("{}".into())).unwrap();
            for (request, value) in [(first, true), (second, false)] {
                assert!(request.starts_with("PATCH /v1/desktop/preferences "));
                let body: serde_json::Value =
                    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
                assert_eq!(body, json!({"work_sidebar_collapsed": value}));
            }
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let first = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
        states.send(app.navigation_sidebar.work_sidebar_collapsed).ok();
        let save = |task| {
            iced_runtime::task::into_stream(task).map(|mut actions| {
                runtime.spawn(async move {
                    while let Some(action) = actions.next().await {
                        if let iced_runtime::Action::Output(message) = action {
                            return message;
                        }
                    }
                    panic!("expected a save completion");
                })
            })
        };
        let first = save(first);
        second_click.recv_timeout(TIMEOUT).unwrap();
        let mut next = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
        states.send(app.navigation_sidebar.work_sidebar_collapsed).ok();

        if let Some(first) = first {
            assert!(iced_runtime::task::into_stream(next).is_none());
            let mut pending = Some(first);
            while let Some(task) = pending {
                let message: Message = runtime
                    .block_on(async { tokio::time::timeout(TIMEOUT, task).await })
                    .unwrap()
                    .unwrap();
                next = app.update(message);
                pending = save(next);
            }
        }
        gateway.join().unwrap();
        assert!(!app.navigation_sidebar.work_sidebar_collapsed);
    });
}
