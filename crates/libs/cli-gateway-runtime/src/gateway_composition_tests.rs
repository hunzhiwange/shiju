use cli_gateway_transport::gateway::{GatewayUiRuntime, GatewaySessionSeed};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use std::{io::{Read, Write}, net::TcpListener, path::PathBuf, thread};

fn spawn_json_sequence_server(
    responses: Vec<(&'static str, &'static str)>,
) -> (u16, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind JSON sequence fixture");
    let port = listener.local_addr().expect("read JSON fixture addr").port();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().expect("accept JSON fixture client");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 512];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).expect("read JSON fixture request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write JSON fixture response");
                String::from_utf8(request).expect("JSON fixture request should be utf8")
            })
            .collect()
    });
    (port, server)
}

fn runtime_for_port(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port))
            .expect("client should construct"),
        GatewaySessionSeed::new(PathBuf::from("/tmp/work")).with_id(Some("runtime-session".into())),
    )
}

#[test]
fn model_picker_excludes_disabled_models_and_preserves_namespaced_ids() {
    let catalog = r#"{
        "all":[{"id":"fixture","name":"Fixture","source":"config","env":[],
        "models":{
            "old":{"id":"old","providerID":"fixture","name":"Old",
                "api":{"id":"old","url":"https://example.com","adapter":"openai-compatible"},
                "capabilities":{"temperature":true,"reasoning":true,"attachment":false,
                    "toolcall":true,
                    "input":{"text":true,"audio":false,"image":false,"video":false,"pdf":false},
                    "output":{"text":true,"audio":false,"image":false,"video":false,"pdf":false},
                    "interleaved":false},
                "cost":{"input":0,"output":0,"cache":{"read":0,"write":0}},
                "limit":{"context":1000,"output":1000},"status":"disabled","release_date":""},
            "vendor/current":{"id":"vendor/current","providerID":"fixture","name":"Current",
                "api":{"id":"vendor/current","url":"https://example.com","adapter":"openai-compatible"},
                "capabilities":{"temperature":true,"reasoning":false,"attachment":false,
                    "toolcall":true,
                    "input":{"text":true,"audio":false,"image":false,"video":false,"pdf":false},
                    "output":{"text":true,"audio":false,"image":false,"video":false,"pdf":false},
                    "interleaved":false},
                "cost":{"input":0,"output":0,"cache":{"read":0,"write":0}},
                "limit":{"context":1000,"output":1000},"status":"active","release_date":""}
        }}],"default":{"fixture":"vendor/current"},"connected":["fixture"]
    }"#;
    let (port, server) = spawn_json_sequence_server(vec![
        ("200 OK", catalog),
        ("200 OK", r#"{"default_model":"fixture/old"}"#),
        ("200 OK", "{}"),
        ("200 OK", "{}"),
    ]);
    let loaded = cli_gateway_capabilities::capabilities::load_session_capabilities_blocking(
        &runtime_for_port(port),
    );
    server.join().expect("join capability fixture");
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.matrix.models, vec!["fixture/vendor/current"]);
    assert_eq!(loaded.default_model.as_deref(), Some("fixture/vendor/current"));
    assert!(!loaded.matrix.supports_thinking);
    assert!(!loaded.matrix.model_efforts.contains_key("fixture/old"));
}
