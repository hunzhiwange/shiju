use super::aggregation_omp::{self as omp, Form};
use std::{
    io::{Read, Write},
    net::TcpListener,
};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint, vw_api_types::agent_apps::aggregation::AggregationQuery,
};

#[tokio::test]
async fn aggregation_t66_aggregation_omp_preserves_contract() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        use std::io::{BufRead, BufReader};
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("POST /v1/app-providers/omp/config/state "));
        let mut length = 0;
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap();
            }
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        let request: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request, serde_json::json!({"targetHostId":"host","projectId":"project"}));
        let body = r#"{"revision":"r1","profile":"中文项目","members":["one","two"],"defaultProviderId":"builtin","defaultModelId":"model"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let state = omp::read(
        &client,
        &AggregationQuery { target_host_id: "host".into(), project_id: "project".into() },
    )
    .await
    .unwrap();
    server.join().unwrap();
    assert_eq!(state.profile.as_deref(), Some("中文项目"));
    assert_eq!(state.default_provider_id.as_deref(), Some("builtin"));
    assert_eq!(state.default_model_id.as_deref(), Some("model"));
    assert_eq!(state.members, vec!["one", "two"]);
    let mut form = Form {
        pending: Some(7),
        id: "one".into(),
        name: "中文模型".into(),
        declaration: iced::widget::text_editor::Content::with_text("providers: &unsafe {}"),
        state: Some(state),
        ..Form::default()
    };
    assert!(form.finish(7, Err("invalid_configuration".into())));
    assert_eq!(form.declaration.text().trim_end(), "providers: &unsafe {}");
    assert_eq!(form.name, "中文模型");
    assert!(form.pending.is_none());
    assert!(!form.finish(7, Ok(form.state.clone().unwrap())));
}

#[tokio::test]
async fn aggregation_t66_write_failure_keeps_yaml_and_allows_correction() {
    use vw_gateway_client::vw_api_types::agent_apps::aggregation::{
        OmpDirectoryChange, OmpDirectoryState,
    };
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let mut form = Form {
        state: Some(OmpDirectoryState {
            revision: "r1".into(),
            profile: Some("work".into()),
            members: vec!["one".into()],
            default_provider_id: Some("builtin".into()),
            default_model_id: Some("native".into()),
        }),
        id: "one".into(),
        name: "中文模型".into(),
        project: Some("project".into()),
        access: 4,
        ..Form::default()
    };
    let raw = "baseUrl: https://example.invalid/v1\napi: openai-completions\nmodels:\n  - id: model-a\n    name: 中文模型";
    form.declaration = iced::widget::text_editor::Content::with_text(raw);
    let (id, body) = form.write(&query, false).unwrap();
    assert!(matches!(body.change, OmpDirectoryChange::Replace { .. }));
    assert_eq!(body.expected_revision, "r1");
    assert_eq!(form.project.as_ref().unwrap().as_ref(), "project");
    assert_eq!(form.access, 4);
    for (status, code) in [
        (422, "invalid_configuration"),
        (409, "revision_conflict"),
        (403, "forbidden"),
        (502, "target_failed"),
        (503, "target_unavailable"),
        (404, "not_found"),
        (422, "unsupported_target"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = GatewayClient::new(GatewayEndpoint::new(
            "127.0.0.1",
            listener.local_addr().unwrap().port(),
        ))
        .unwrap();
        let expected = serde_json::to_value(&body).unwrap();
        let server = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("PUT /v1/app-providers/omp/one/directory-entry "));
            let mut length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap();
                }
            }
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual, expected);
            assert!(actual.get("mcpServers").is_none());
            assert!(actual.get("login").is_none());
            let response =
                serde_json::json!({"error":{"code":code,"message":"sensitive upstream detail"}})
                    .to_string();
            write!(stream,"HTTP/1.1 {status} Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
        });
        form.pending = Some(1);
        let result = omp::execute(client, query.clone(), Some((id.clone(), body.clone()))).await;
        assert!(result.is_err());
        assert!(!result.as_ref().unwrap_err().contains("sensitive"));
        if code == "unsupported_target" {
            assert!(result.as_ref().unwrap_err().contains("不支持"));
        }
        assert!(form.finish(1, result));
        server.join().unwrap();
        assert_eq!(form.declaration.text().trim_end(), raw);
        assert_eq!(form.id, "one");
        // Retry remains an explicit user action, with the original revision.
        assert_eq!(form.write(&query, false).unwrap().1.expected_revision, "r1");
    }
    form.declaration = iced::widget::text_editor::Content::with_text("models: [");
    assert!(form.write(&query, false).is_err());
    form.declaration = iced::widget::text_editor::Content::with_text(raw);
    form.id = "new".into();
    assert!(matches!(
        form.write(&query, false).unwrap().1.change,
        OmpDirectoryChange::Insert { .. }
    ));
    form.removing = Some("one".into());
    assert!(matches!(form.write(&query, true).unwrap().1.change, OmpDirectoryChange::Remove {}));
    form.id = "../bad".into();
    assert!(form.write(&query, false).is_err());
}

#[tokio::test]
async fn aggregation_t66_success_keeps_default_profile_and_advances_revision() {
    use vw_gateway_client::vw_api_types::agent_apps::aggregation::OmpDirectoryState;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let before = OmpDirectoryState {
        revision: "old".into(),
        profile: None,
        members: vec![],
        default_provider_id: Some("builtin".into()),
        default_model_id: Some("native".into()),
    };
    let mut form = Form {
        state: Some(before.clone()),
        id: "new".into(),
        name: "新模型".into(),
        declaration: iced::widget::text_editor::Content::with_text("models: []"),
        pending: Some(2),
        ..Form::default()
    };
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let write = form.write(&query, false).unwrap();
    let mut after = before;
    after.revision = "new".into();
    after.members.push("new".into());
    let response = after.clone();
    let server = std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("PUT /v1/app-providers/omp/new/directory-entry "));
        let mut length = 0;
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap();
            }
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["projectId"], "project");
        assert_eq!(body["expectedRevision"], "old");
        assert_eq!(body["change"]["kind"], "insert");
        let response = serde_json::to_string(&response).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        )
        .unwrap();
    });
    let result = omp::execute(client, query, Some(write)).await;
    server.join().unwrap();
    assert!(form.finish(2, result));
    assert_eq!(form.state, Some(after));
    assert_eq!(form.declaration.text().trim_end(), "models: []");
}
