use super::hermes;
use crate::app::state::{
    AgentAppsContext,
    app_providers::{
        edit::{EditForm, Mode},
        hermes::Draft,
        list::{ListState, Scope},
    },
};
use iced::widget::text_editor::Content;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{basic::ProviderBasic, hermes::HermesLiveResult},
    },
};

const INVALID: &str = r#"{"models":[{"id":"","name":"中文模型"}]}"#;

fn client_scope(port: u16) -> (GatewayClient, Scope) {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Hermes,
    };
    (client, scope)
}

fn draft() -> EditForm {
    let mut form = EditForm::new(
        Mode::Create,
        "fixture".into(),
        ProviderBasic {
            name: "中文草稿".into(), category: None, notes: None, website_url: None
        },
        None,
    );
    form.hermes = Some(Draft { declaration: Content::with_text(INVALID), ..Draft::new() });
    form.saving = true;
    form
}

#[tokio::test]
async fn cc04_ui_hermes_validation_failure_preserves_draft() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (client, scope) = client_scope(listener.local_addr().unwrap().port());
    let mut state = ListState::default();
    let generation = state.begin(scope.clone());
    let form = draft();
    state.form = Some(form.clone());
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= Duration::from_secs(4) {
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("POST /v1/app-providers/hermes/fixture/form "));
        let mut length = 0;
        let mut authorized = false;
        loop {
            line.clear();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            let header = line.to_ascii_lowercase();
            if let Some(value) = header.strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            authorized |= header.trim_end() == "x-agent-apps-token: fake-host-token";
        }
        assert!(authorized);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["targetHostId"], "fixture-host");
        assert_eq!(body["fields"]["name"], "中文草稿");
        assert_eq!(body["declaration"].as_str().unwrap().trim_end(), INVALID);
        stream.write_all(b"HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        true
    });
    let result = hermes::save(client, scope.clone(), form, "fake-host-token".into()).await;
    assert!(server.join().unwrap(), "Hermes 草稿必须通过 SDK 交给执行端校验");
    assert!(result.as_ref().unwrap_err().contains("执行端未接受"));
    assert!(hermes::finish(&mut state, &scope, generation, &result));
    let form = state.form.unwrap();
    assert!(!form.saving);
    assert_eq!(form.fields.name, "中文草稿");
    assert_eq!(form.hermes.unwrap().declaration.text().trim_end(), INVALID);
}

#[test]
fn cc04_ui_hermes_stale_response_is_ignored() {
    let (_, scope) = client_scope(1);
    let mut state = ListState::default();
    let old = state.begin(scope.clone());
    let current = state.begin(scope.clone());
    let mut form = draft();
    form.fields.name = "当前草稿".into();
    state.form = Some(form);

    assert!(!hermes::finish(&mut state, &scope, old, &Ok(HermesLiveResult::Applied)));
    let mut old_scope = scope.clone();
    old_scope.host = "old-host".into();
    assert!(!hermes::finish(&mut state, &old_scope, current, &Err("过期错误".into())));
    assert_eq!(state.form.as_ref().unwrap().fields.name, "当前草稿");
    assert!(state.form.as_ref().unwrap().saving);
    assert!(hermes::finish(&mut state, &scope, current, &Ok(HermesLiveResult::Applied)));
    assert!(state.form.is_none());
}
