use super::codex;
use crate::app::state::{
    AgentAppsContext,
    app_providers::{
        codex::Draft,
        edit::{EditForm, Mode},
        list::{ListState, Scope},
    },
};
use std::io::{Read, Write};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{agent_apps::AppId, app_providers::basic::ProviderBasic},
};

fn client_scope(port: u16) -> (GatewayClient, Scope) {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Codex,
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
    let mut draft = Draft::new();
    draft.toml = iced::widget::text_editor::Content::with_text("model = [ # 无效草稿");
    form.codex = Some(draft);
    form
}

#[tokio::test]
async fn cc04_ui_codex_validation_failure_preserves_draft() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let (client, scope) = client_scope(listener.local_addr().unwrap().port());
    let mut form = draft();
    form.key = "fake-form-key".into();
    super::codex::change(&mut form, super::codex::Field::AddModel);
    super::codex::change(&mut form, super::codex::Field::ModelId(0, "fixture-model".into()));
    super::codex::change(&mut form, super::codex::Field::ModelName(0, "保留模型".into()));
    form.saving = true;
    let mut state = ListState::default();
    let generation = state.begin(scope.clone());
    state.form = Some(form.clone());
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed().as_secs() >= 4 {
                        return false;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(4))).unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).unwrap();
        assert!(head.starts_with("POST /v1/app-providers/codex/fixture/form "));
        assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
        let len = head
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length:")
                    .map(|s| s.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["fields"]["name"], "中文草稿");
        assert_eq!(body["targetHostId"], "fixture-host");
        assert_eq!(body["toml"].as_str().unwrap().trim_end(), "model = [ # 无效草稿");
        assert_eq!(body["apiKey"]["value"], "fake-form-key");
        assert_eq!(body["models"][0]["slug"], "fixture-model");
        write!(
            stream,
            "HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        true
    });

    assert!(!format!("{form:?}").contains("fake-form-key"));
    let result = codex::save(client, scope.clone(), form, "fake-host-token".into()).await;
    assert!(server.join().unwrap(), "Codex 草稿必须通过正式 SDK 提交给服务校验");
    assert!(result.as_ref().unwrap_err().contains("执行端未接受"));
    assert!(codex::finish(&mut state, &scope, generation, &result));
    let form = state.form.unwrap();
    assert_eq!(form.fields.name, "中文草稿");
    let draft = form.codex.unwrap();
    assert_eq!(draft.toml.text().trim_end(), "model = [ # 无效草稿");
    assert_eq!(draft.models.unwrap()[0].display_name, "保留模型");
    assert_eq!(form.key, "fake-form-key");
    assert!(!form.saving);
}

#[test]
fn cc04_ui_codex_stale_response_is_ignored() {
    use vw_gateway_client::vw_api_types::app_providers::codex_catalog::CodexLiveResult;
    let (_, scope) = client_scope(1);
    let mut state = ListState::default();
    let old_generation = state.begin(scope.clone());
    let generation = state.begin(scope.clone());
    let mut new = draft();
    new.fields.name = "新草稿".into();
    new.saving = true;
    state.form = Some(new);

    assert!(!codex::finish(&mut state, &scope, old_generation, &Ok(CodexLiveResult::Applied)));
    let mut old_scope = scope.clone();
    old_scope.host = "old-host".into();
    assert!(!codex::finish(&mut state, &old_scope, generation, &Err("过期错误".into())));
    assert_eq!(state.form.as_ref().unwrap().fields.name, "新草稿");
    assert!(state.form.as_ref().unwrap().saving);
    assert!(codex::finish(&mut state, &scope, generation, &Ok(CodexLiveResult::NotAttempted)));
    assert!(state.form.is_none());
}
