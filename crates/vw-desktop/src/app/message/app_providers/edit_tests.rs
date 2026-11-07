use crate::app::state::app_providers::edit::*;
use vw_gateway_client::vw_api_types::app_providers::basic::ProviderBasic;
fn form(mode: Mode) -> EditForm {
    EditForm::new(
        mode,
        "fixture".into(),
        ProviderBasic {
            name: "中文供应商".into(),
            category: None,
            notes: None,
            website_url: Some("https://website.example.test".into()),
        },
        None,
    )
}
#[test]
fn cc04_ui_edit_redacted_secret_is_not_submitted() {
    assert!(
        !format!("{:?}", super::EditMessage::Key("sensitive-fixture-key".into()))
            .contains("sensitive-fixture-key")
    );
    let mut form = form(Mode::Update);
    form.replace_key = true;
    for marker in ["[redacted]", "<redacted>", "••••", "****"] {
        form.key = marker.into();
        assert!(form.request().is_err(), "脱敏值不得作为原始密钥提交: {marker}");
    }
}
#[test]
fn cc04_ui_edit_delete_error_preserves_selection() {
    let mut form = form(Mode::Delete);
    form.saving = true;
    assert!(!form.finish(&Err("403".into())), "删除失败不能关闭选择与确认");
    assert_eq!(form.id, "fixture");
    assert!(!form.saving);
}
#[test]
fn cc04_ui_edit_missing_draft_disables_create_and_keep_has_no_secret_value() {
    let mut create = form(Mode::Create);
    create.key = "fixture-key".into();
    assert!(create.request().unwrap_err().contains("完整应用配置"));
    let mut edit = form(Mode::Update);
    edit.key = "[redacted]".into();
    let value = serde_json::to_value(edit.request().unwrap()).unwrap();
    assert_eq!(value["apiKey"], serde_json::json!({"action":"keep"}));
    assert!(value.get("model").is_none());
    assert!(value.get("baseUrl").is_none());
}
#[test]
fn cc04_ui_edit_copy_uses_new_identity_and_explicit_clear_is_preserved() {
    let mut copy = form(Mode::Copy { source_id: "source".into() });
    let value = serde_json::to_value(copy.request().unwrap()).unwrap();
    assert_eq!(value["action"]["source_id"], "source");
    assert_eq!(value["apiKey"]["action"], "keep");
    copy.clear_key = true;
    assert_eq!(serde_json::to_value(copy.request().unwrap()).unwrap()["apiKey"]["action"], "clear");
}
#[tokio::test]
async fn cc04_ui_edit_save_dispatches_keep_and_delete_errors_through_sdk() {
    use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
    use std::io::{Read, Write};
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for delete in [false, true] {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                header.push(b[0]);
            }
            let header = String::from_utf8(header).unwrap();
            let length: usize = header
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|s| s.trim().parse().unwrap())
                })
                .unwrap();
            let mut bytes = vec![0; length];
            stream.read_exact(&mut bytes).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let (status, body) = if delete {
                assert!(header.starts_with("DELETE /v1/app-providers/claude/fixture "));
                assert!(header.to_lowercase().contains("x-agent-apps-token: fake-token"));
                assert_eq!(value["targetHostId"], "fixture-host");
                ("403 Forbidden", serde_json::json!({"message":"fixture denial"}))
            } else {
                assert!(header.starts_with("PUT /v1/app-providers/claude/fixture/basic "));
                assert_eq!(value["apiKey"], serde_json::json!({"action":"keep"}));
                assert_eq!(value["fields"]["websiteUrl"], "https://website.example.test");
                assert!(value.get("model").is_none());
                ("200 OK", value["fields"].clone())
            };
            let body = body.to_string();
            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", address.port())).unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: vw_gateway_client::vw_api_types::agent_apps::AppId::Claude,
    };
    super::save(client.clone(), scope.clone(), form(Mode::Update), "fake-token".into())
        .await
        .unwrap();
    let mut delete = form(Mode::Delete);
    delete.saving = true;
    let result = super::save(client, scope, delete.clone(), "fake-token".into()).await;
    assert!(result.is_err());
    assert!(!delete.finish(&result));
    assert_eq!(delete.id, "fixture");
    server.join().unwrap();
}
#[test]
fn cc04_ui_edit_create_never_replays_credentials_embedded_in_draft() {
    use vw_gateway_client::vw_api_types::app_providers::{ProviderProtocol, create::*};
    let mut create = form(Mode::Create);
    create.draft = Some(AppProviderCreate {
        name: "draft".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: ProviderSecretWrite::Replace { value: "unrequested-draft-secret".into() },
        base_url: "https://api.example.test".into(),
        model: "fixture-model".into(),
        request_options: None,
        add_to_live: false,
    });
    create.key = "explicit-new-key".into();
    let value = serde_json::to_value(create.request().unwrap()).unwrap();
    assert!(!value.to_string().contains("unrequested-draft-secret"));
    assert_eq!(value["apiKey"]["value"], "explicit-new-key");
    assert_eq!(value["action"]["draft"]["apiKey"]["action"], "keep");
    create.draft.as_mut().unwrap().model.clear();
    assert!(!create.draft_ready());
    assert!(create.request().is_err());
}
