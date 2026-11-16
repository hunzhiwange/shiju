use crate::access::run_gateway_call;
use crate::config_desktop::{
    DesktopPreferencesPersistence, PatchTarget, PreferencesMessage as Message,
};
use gateway_endpoint::GatewayEndpoint;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

fn endpoint(port: u16) -> GatewayEndpoint {
    GatewayEndpoint::new("127.0.0.1", port)
}
fn access(port: u16) -> crate::ConfigAccess {
    crate::ConfigAccess::new(endpoint(port), None)
}
#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    body: String,
}
struct TestServer {
    port: u16,
    handle: JoinHandle<Vec<RecordedRequest>>,
}
impl TestServer {
    fn start(
        count: usize,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            (0..count).map(|_| {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap(); bytes.push(byte[0]);
            }
            let headers = String::from_utf8(bytes).unwrap();
            let mut parts = headers.lines().next().unwrap().split_whitespace();
            let method = parts.next().unwrap().into();
            let path = parts.next().unwrap().into();
            let len = headers.lines().filter_map(|line| line.split_once(':')).find(|(key, _)| key.eq_ignore_ascii_case("content-length")).map(|(_, value)| value.trim().parse().unwrap()).unwrap_or(0);
            let mut body = vec![0; len]; stream.read_exact(&mut body).unwrap();
            let request = RecordedRequest { method, path, body: String::from_utf8(body).unwrap() };
            let (status, body) = handler(&request);
            write!(stream, "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).unwrap();
            request
        }).collect()
        });
        Self { port, handle }
    }
    fn take_requests(self) -> Vec<RecordedRequest> {
        self.handle.join().unwrap()
    }
}

async fn collect_preference_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return Vec::new();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut messages = Vec::new();
        while let Some(action) = stream.next().await {
            if let iced_runtime::Action::Output(message) = action {
                messages.push(message);
            }
        }
        messages
    })
    .await
    .expect("preference tasks should finish")
}
fn finish(state: &mut DesktopPreferencesPersistence, message: Message) -> iced::Task<Message> {
    match message {
        Message::DesktopPreferencesSaved(scope, _) => state.finished(scope, PatchTarget::Desktop),
        Message::ProjectChatPreferencesSaved(scope, _) => {
            let task = state.project_chat.finished(scope);
            state.notify_if_idle();
            task
        }
        _ => panic!("unexpected save message"),
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn preference_save_failure_keeps_new_edits_and_preserves_unrelated_fields() {
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::time::Duration;

    let mut state = DesktopPreferencesPersistence::default();
    let stored = Arc::new(Mutex::new(json!({"theme": "dark", "obsolete": true})));
    let server_state = stored.clone();
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        let fields: Value = serde_json::from_str(&request.body).unwrap();
        if fields == json!({"show_settings": true}) {
            return (500, r#"{"error":"preference rejected"}"#);
        }
        assert_eq!(fields, json!({"show_settings": false, "obsolete": null}));
        let mut stored = server_state.lock().unwrap();
        let stored = stored.as_object_mut().unwrap();
        for (key, value) in fields.as_object().unwrap() {
            if value.is_null() {
                stored.remove(key);
            } else {
                stored.insert(key.clone(), value.clone());
            }
        }
        (200, "{}")
    });
    let endpoint = endpoint(server.port);
    let receive_save = |task| {
        run_gateway_call(async {
            let mut actions = iced_runtime::task::into_stream(task).expect("save task");
            match tokio::time::timeout(Duration::from_secs(4), actions.next()).await {
                Ok(Some(iced_runtime::Action::Output(
                    message @ Message::DesktopPreferencesSaved(..),
                ))) => Ok(message),
                _ => Err("missing desktop preference save result".to_string()),
            }
        })
        .unwrap()
    };

    let first =
        state.enqueue(endpoint.clone(), json!({"show_settings":true}).as_object().unwrap().clone());
    let queued = state.enqueue(
        endpoint.clone(),
        json!({"show_settings": false, "obsolete": null}).as_object().unwrap().clone(),
    );
    assert!(iced_runtime::task::into_stream(queued).is_none());
    let failed = receive_save(first);
    assert!(matches!(&failed, Message::DesktopPreferencesSaved(_, Err(_))));
    let Message::DesktopPreferencesSaved(scope, _) = failed else {
        panic!("desktop save");
    };
    let second = state.finished(scope, PatchTarget::Desktop);

    let saved = receive_save(second);
    assert!(matches!(&saved, Message::DesktopPreferencesSaved(_, Ok(()))));
    let Message::DesktopPreferencesSaved(scope, _) = saved else {
        panic!("desktop save");
    };
    let done = state.finished(scope, PatchTarget::Desktop);
    assert!(iced_runtime::task::into_stream(done).is_none());
    assert_eq!(*stored.lock().unwrap(), json!({"theme": "dark", "show_settings": false}));
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn project_puts_keep_the_original_gateway_and_drain_after_a_failed_save() {
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let mut state = DesktopPreferencesPersistence::default();
    let writes = AtomicUsize::new(0);
    let first_gateway = TestServer::start(4, move |request| match request.method.as_str() {
        "PATCH" => (200, "{}"),
        "PUT" if writes.fetch_add(1, Ordering::SeqCst) == 0 => (500, "{}"),
        "PUT" => (200, "{}"),
        _ => panic!("unexpected desktop request {request:?}"),
    });
    let second_gateway = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        (200, "{}")
    });
    let first_endpoint = endpoint(first_gateway.port);
    let initial = ("auto".into(), true, None);

    let first = state.save_chat_model_preferences(
        first_endpoint.clone(),
        Some("/repo/a".into()),
        initial.clone(),
        ("first".into(), false, None),
    );
    let next = state.save_chat_model_preferences(
        first_endpoint,
        Some("/repo/a".into()),
        initial.clone(),
        ("last".into(), false, None),
    );
    let idle = state.wait_for_idle().unwrap();
    assert!(!*idle.borrow());

    let other = state.save_chat_model_preferences(
        endpoint(second_gateway.port),
        None,
        initial,
        ("other".into(), false, None),
    );
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        assert!(collect_preference_messages(next).await.is_empty());
        let messages = collect_preference_messages(other).await;
        assert_eq!(messages.len(), 1);
        for message in messages {
            let _ = finish(&mut state, message);
        }
        assert!(!*idle.borrow());
        let messages = collect_preference_messages(first).await;
        assert_eq!(messages.len(), 2);
        let follow_up =
            iced::Task::batch(messages.into_iter().map(|message| finish(&mut state, message)));
        assert!(!*idle.borrow());
        // Host Toast rendering is covered by its separate boundary test.
        let messages = collect_preference_messages(follow_up).await;
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(
                    message,
                    Message::DesktopPreferencesSaved(..) | Message::ProjectChatPreferencesSaved(..)
                ))
                .count(),
            2
        );
        for message in messages {
            let _ = finish(&mut state, message);
        }
    });

    assert!(*idle.borrow());
    assert_eq!(state.project_chat.global.as_ref().unwrap().1.0, "other");
    let requests = first_gateway.take_requests();
    let projects: Vec<_> = requests.iter().filter(|request| request.method == "PUT").collect();
    assert_eq!(projects.len(), 2);
    for (request, model) in projects.into_iter().zip(["first", "last"]) {
        assert_eq!(request.path, "/v1/desktop/project-preferences?project_path=%2Frepo%2Fa");
        assert_eq!(
            serde_json::from_str::<Value>(&request.body).unwrap(),
            json!({"model":model,"auto_model":false,"acp_agent":null})
        );
    }
    assert_eq!(second_gateway.take_requests().len(), 1);
}

#[test]
fn reasoning_and_send_preferences_preserve_unrelated_sessions_and_remove_legacy_key() {
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::sync::mpsc;
    use std::time::Duration;

    let mut state = DesktopPreferencesPersistence::default();
    let (release_send_save, allow_send_save) = mpsc::channel();
    let allow_send_save = Mutex::new(allow_send_save);
    let server = TestServer::start(3, move |request| {
        assert_eq!(request.path, "/v1/desktop/preferences");
        match request.method.as_str() {
            "GET" => (200, r#"{"session_reasoning_efforts":{"unopened":"future","one":"high"}}"#),
            "PATCH" => {
                if request.body.contains("chat_send_behavior") {
                    allow_send_save.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
                }
                (200, "{}")
            }
            _ => panic!("unexpected desktop request {request:?}"),
        }
    });
    let endpoint = endpoint(server.port);
    let behavior = state.enqueue(
        endpoint.clone(),
        json!({"chat_send_behavior":"queue","dialogue_flow_follow_up_behavior":null})
            .as_object()
            .unwrap()
            .clone(),
    );
    let _ = state.enqueue(
        endpoint.clone(),
        json!({"session_reasoning_efforts":{"one":null}}).as_object().unwrap().clone(),
    );
    let _ = state.enqueue(
        endpoint,
        json!({"session_reasoning_efforts":{"two":"high"}}).as_object().unwrap().clone(),
    );

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut actions = iced_runtime::task::into_stream(behavior).unwrap();
        release_send_save.send(()).unwrap();
        let saved = tokio::time::timeout(Duration::from_secs(5), actions.next()).await.unwrap();
        let Some(iced_runtime::Action::Output(save @ Message::DesktopPreferencesSaved(..))) = saved
        else {
            panic!("missing send preference save completion");
        };
        let reasoning = finish(&mut state, save);
        let messages = collect_preference_messages(reasoning).await;
        assert_eq!(messages.len(), 1);
        let _ = finish(&mut state, messages.into_iter().next().unwrap());
    });
    let requests = server.take_requests();
    assert_eq!(
        requests.iter().map(|req| req.method.as_str()).collect::<Vec<_>>(),
        ["PATCH", "GET", "PATCH"]
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[0].body).unwrap(),
        json!({"chat_send_behavior":"queue","dialogue_flow_follow_up_behavior":null})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap(),
        json!({"session_reasoning_efforts":{"unopened":"future","one":null,"two":"high"}})
    );
}

const SOURCE: &str = include_str!("../config_desktop.rs");
fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn config_desktop_tests_keeps_planned_coverage_targets() {
    for name in [
        "load_app_config_async",
        "save_app_config_async",
        "update_agents_compat_registry_result_async",
        "update_agents_compat_registry_async",
        "load_app_config",
        "save_app_config",
        "set_config_field",
        "load_project_chat_preferences",
        "save_project_chat_preferences",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
