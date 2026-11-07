use super::snippets::{finish, save};
use crate::app::state::{
    AgentAppsContext,
    app_providers::{
        list::Scope,
        snippets::{Form, Operation},
    },
};
use iced::widget::text_editor::Content;
use std::io::{BufRead, BufReader, Read, Write};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{agent_apps::AppId, app_providers::snippets::ConfigSnippet},
};

const INVALID: &str = "{ 中文草稿";
const TOKEN: &str = "fixture-host-token";

fn scope(client: &GatewayClient) -> Scope {
    Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Claude,
    }
}

fn draft() -> Form {
    Form {
        draft: Content::with_text(INVALID),
        pending: Some(Operation::Save),
        loaded: true,
        ..Form::default()
    }
}

#[tokio::test]
async fn cc04_ui_snippets_validation_failure_preserves_draft() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let scope = scope(&client);
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "PUT /v1/app-providers/claude/config/snippet HTTP/1.1\r\n");
        let mut len = 0;
        let mut token = None;
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => len = value.trim().parse().unwrap(),
                    "x-agent-apps-token" => token = Some(value.trim().to_owned()),
                    _ => {}
                }
            }
        }
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(token.as_deref(), Some(TOKEN));
        assert_eq!(body["targetHostId"], "fixture-host");
        assert_eq!(body["content"].as_str().unwrap().trim_end(), INVALID);
        write!(
            stream,
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
    });
    let mut form = draft();
    let content = form.draft.text();
    let result = save(&client, &scope, TOKEN, content.clone()).await;
    server.join().unwrap();
    assert!(finish(&mut form, (&scope, 3), (&scope, 3), result));
    assert_eq!(form.draft.text(), content);
    assert!(form.error.as_deref().is_some_and(|error| error.contains("400")));
    assert!(form.pending.is_none());
}

#[test]
fn cc04_ui_snippets_stale_response_is_ignored() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let scope = scope(&client);
    let mut other = scope.clone();
    other.context.project = Some("/other-project".into());
    let mut form = draft();
    form.pending = Some(Operation::Preview);
    let original = form.draft.text();
    let value = ConfigSnippet { content: Some(r#"{"theme":"中文预览"}"#.into()) };
    for completed in [(&scope, 3), (&other, 4)] {
        assert!(!finish(&mut form, (&scope, 4), completed, Ok(value.clone())));
        assert_eq!(form.draft.text(), original);
        assert_eq!(form.pending, Some(Operation::Preview));
        assert!(form.error.is_none());
        assert!(form.preview.is_none());
    }
    assert!(finish(&mut form, (&scope, 4), (&scope, 4), Ok(value.clone())));
    assert_eq!(form.draft.text(), original);
    assert_eq!(form.preview, Some((Operation::Preview, value)));
    assert!(form.pending.is_none());
}
