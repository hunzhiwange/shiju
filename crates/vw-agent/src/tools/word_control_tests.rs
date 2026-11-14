use super::word_control::*;
use super::{Tool, ToolUseContext, execute_tool_from_registry};
use crate::computer_control::{
    office::{sessions::OfficeSurface, transport::OfficeTransport},
    policy::{ComputerControlPolicy, ControlFeature, GrantScope},
};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use rustls_pki_types::{CertificateDer, ServerName, pem::PemObject};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::GrantDecision;

type Client = tokio_tungstenite::WebSocketStream<tokio_rustls::client::TlsStream<TcpStream>>;

fn certificates() -> &'static tempfile::TempDir {
    static CERTS: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    CERTS.get_or_init(|| {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=localhost",
                "-addext",
                "subjectAltName=DNS:localhost",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-keyout",
            ])
            .arg(dir.path().join("key.pem"))
            .arg("-out")
            .arg(dir.path().join("cert.pem"))
            .output()
            .unwrap();
        assert!(output.status.success());
        for name in ["key.pem", "cert.pem"] {
            std::fs::set_permissions(dir.path().join(name), std::fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        dir
    })
}

async fn client(host: &OfficeTransport, origin: &str) -> anyhow::Result<Client> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from_pem_file(certificates().path().join("cert.pem"))?)?;
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let tcp = TcpStream::connect(host.address()).await?;
    let tls = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("localhost")?, tcp)
        .await?;
    let mut req =
        format!("wss://localhost:{}/office", host.address().port()).into_client_request()?;
    req.headers_mut().insert("Origin", origin.parse()?);
    Ok(tokio_tungstenite::client_async(req, tls).await?.0)
}

struct Fixture {
    runtime: Arc<super::computer_control::NativeControlRuntime>,
    transport: Arc<OfficeTransport>,
    tools: Vec<Box<dyn Tool>>,
    context: Arc<ToolUseContext>,
}
impl Fixture {
    async fn new() -> Self {
        let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));
        policy.write().set_enabled(&local_user(), ControlFeature::Word, true).unwrap();
        let runtime = super::computer_control::NativeControlRuntime::with_factory(
            policy,
            Arc::new(|_, _, _, _, _, _| panic!("Word must never use native backend")),
        );
        let transport =
            runtime.bind_office(certificates().path(), "https://localhost:3000").await.unwrap();
        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(WordControlTool::new(runtime.clone(), Some("chat".into())))];
        Self { runtime, transport, tools, context: Arc::new(ToolUseContext::new("chat", None)) }
    }
    async fn pair(&self, document: &str, capabilities: &[&str]) -> Client {
        self.runtime
            .policy
            .write()
            .set_grant(
                &local_user(),
                GrantScope::WordDocument(document.into()),
                Some(GrantDecision::Allow),
            )
            .unwrap();
        let token = self
            .transport
            .approve_pairing(
                office_owner(&self.context).unwrap(),
                OfficeSurface::Word,
                ControlDocumentId(document.into()),
            )
            .unwrap();
        let mut socket = client(&self.transport, "https://localhost:3000").await.unwrap();
        socket.send(Message::text(json!({"type":"pair","token":token,"surface":"word","document_id":document,"title":"Untitled","capabilities":capabilities}).to_string())).await.unwrap();
        assert_eq!(receive(&mut socket).await["type"], "paired");
        socket
    }
    async fn call(&self, input: Value) -> Result<Value, super::toolset::ToolCallError> {
        execute_tool_from_registry(&self.tools, "word_control", input, self.context.clone())
            .await
            .map(|r| r.result.data)
    }
    fn request(&self, document: &str, command: Value) -> Value {
        let owner = office_owner(&self.context).unwrap();
        let session = self
            .transport
            .documents(&owner)
            .into_iter()
            .find(|s| s.handle.document_id.0 == document)
            .unwrap();
        json!({"operation":"execute","document_id":document,"generation":session.handle.generation.to_string(),"command":command})
    }
}
async fn receive(socket: &mut Client) -> Value {
    let message = tokio::time::timeout(Duration::from_secs(3), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(message.to_text().unwrap()).unwrap()
}
fn read() -> Value {
    json!({"kind":"read_document"})
}

#[tokio::test]
async fn cc_control_16_no_session_discovers_empty_and_refuses_execution() {
    let f = Fixture::new().await;
    assert_eq!(f.call(json!({"operation":"discover"})).await.unwrap()["sessions"], json!([]));
    assert!(f.call(json!({"operation":"execute","document_id":"missing","generation":uuid::Uuid::new_v4().to_string(),"command":read()})).await.is_err());
}
#[tokio::test]
async fn cc_control_16_duplicate_titles_require_exact_document_and_generation() {
    let f = Fixture::new().await;
    let _a = f.pair("first", &["word_read_document"]).await;
    let _b = f.pair("second", &["word_read_document"]).await;
    let discovered = f.call(json!({"operation":"discover"})).await.unwrap();
    assert_eq!(discovered["sessions"].as_array().unwrap().len(), 2);
    assert!(
        f.call(json!({"operation":"execute","title":"Untitled","command":read()})).await.is_err()
    );
    let mut request = f.request("first", read());
    request["generation"] = f.request("second", read())["generation"].clone();
    assert!(f.call(request).await.is_err());
}
#[tokio::test]
async fn cc_control_16_switch_and_document_grant_both_gate_execution() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    let request = f.request("book", read());
    f.runtime.policy.write().set_enabled(&local_user(), ControlFeature::Word, false).unwrap();
    assert!(f.call(request.clone()).await.is_err());
    f.runtime.policy.write().set_enabled(&local_user(), ControlFeature::Word, true).unwrap();
    for grant in [None, Some(GrantDecision::Deny)] {
        f.runtime
            .policy
            .write()
            .set_grant(&local_user(), GrantScope::WordDocument("book".into()), grant)
            .unwrap();
        assert!(f.call(request.clone()).await.is_err());
    }
    assert!(tokio::time::timeout(Duration::from_millis(20), socket.next()).await.is_err());
}
#[tokio::test]
async fn cc_control_16_missing_capability_and_unknown_fields_are_rejected() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    assert!(
        f.call(f.request(
            "book",
            json!({"kind":"replace_text","search":"before","replacement":"after"})
        ))
        .await
        .is_err()
    );
    let mut request = f.request("book", read());
    request["script"] = json!("arbitrary");
    assert!(f.call(request).await.is_err());
    assert!(f.call(f.request("book", json!({"kind":"run_macro","name":"bad"}))).await.is_err());
    assert!(tokio::time::timeout(Duration::from_millis(20), socket.next()).await.is_err());
}
#[tokio::test]
async fn cc_control_16_replace_returns_actual_changed_range() {
    let f = Fixture::new().await;
    let mut socket = f.pair("doc", &["word_replace_text"]).await;
    let request = f.request("doc", json!({"kind":"replace_text","search":"before","replacement":"after"}));
    let responder = tokio::spawn(async move {
        let command = receive(&mut socket).await;
        assert_eq!(command["action"]["command"]["search"], "before");
        socket.send(Message::text(json!({"type":"result","sequence":1,"request_id":command["request_id"],"value":{"revision":1,"text":"after","paragraph":"after paragraph","previous":"before","bold":null}}).to_string())).await.unwrap();
        receive(&mut socket).await;
        socket
    });
    let result = f.call(request.clone()).await.unwrap();
    assert_eq!(result["result"]["previous"], "before");
    assert_eq!(result["result"]["text"], "after");
    assert_eq!(result["generation"], request["generation"]);
    let _socket = responder.await.unwrap();
}
#[tokio::test]
async fn cc_control_16_disconnect_and_repair_invalidate_old_generation() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    let old = f.request("book", read());
    socket.close(None).await.unwrap();
    for _ in 0..100 {
        if f.transport.documents(&office_owner(&f.context).unwrap()).is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(f.call(old.clone()).await.is_err());
    let _new = f.pair("book", &["word_read_document"]).await;
    assert!(f.call(old).await.is_err());
}
#[tokio::test]
async fn cc_control_16_foreign_executor_owner_cannot_discover_or_execute() {
    let f = Fixture::new().await;
    let _socket = f.pair("book", &["word_read_document"]).await;
    assert!(
        execute_tool_from_registry(
            &f.tools,
            "word_control",
            json!({"operation":"discover"}),
            Arc::new(ToolUseContext::new("other", None))
        )
        .await
        .is_err()
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(WordControlTool::new(f.runtime.clone(), None))];
    let result = execute_tool_from_registry(
        &tools,
        "word_control",
        json!({"operation":"discover"}),
        Arc::new(ToolUseContext::new("other", None)),
    )
    .await
    .unwrap();
    assert_eq!(result.result.data["sessions"], json!([]));
}
#[tokio::test]
async fn cc_control_16_revocation_during_command_discards_result() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    let request = f.request("book", read());
    let runtime = f.runtime.clone();
    let responder = tokio::spawn(async move {
        let command = receive(&mut socket).await;
        runtime.policy.write().set_enabled(&local_user(), ControlFeature::Word, false).unwrap();
        socket.send(Message::text(json!({"type":"result","sequence":1,"request_id":command["request_id"],"value":{"values":[[42]]}}).to_string())).await.unwrap();
        let _ = socket.next().await;
        socket
    });
    assert!(f.call(request).await.unwrap_err().to_string().contains("StalePermission"));
    let _socket = responder.await.unwrap();
}

#[tokio::test]
async fn cc_control_16_invalid_text_never_dispatches() {
    let f = Fixture::new().await;
    let mut socket = f.pair("doc", &["word_find_text"]).await;
    for text in [String::new(), "line\nbreak".into(), "x".repeat(8193)] {
        assert!(f.call(f.request("doc", json!({"kind":"find_text","text":text}))).await.is_err());
    }
    assert!(tokio::time::timeout(Duration::from_millis(20), socket.next()).await.is_err());
}

#[tokio::test]
async fn cc_control_16_range_conflict_is_failure_and_invalidates_session() {
    let f = Fixture::new().await;
    let mut socket = f.pair("doc", &["word_replace_text"]).await;
    let request = f.request("doc", json!({"kind":"replace_text","search":"before","replacement":"after"}));
    let responder = tokio::spawn(async move {
        let command = receive(&mut socket).await;
        socket.send(Message::text(json!({"type":"result","sequence":1,"request_id":command["request_id"],"value":{"revision":1,"text":"after","paragraph":"after","previous":"different"}}).to_string())).await.unwrap();
        let _ = socket.next().await;
    });
    assert!(f.call(request.clone()).await.is_err());
    responder.await.unwrap();
    assert!(f.call(request).await.is_err());
}

#[tokio::test]
async fn cc_control_16_abort_invalidates_connection_and_discards_late_result() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    let request = f.request("book", read());
    let cancel = tokio_util::sync::CancellationToken::new();
    let context = Arc::new(f.context.as_ref().clone().with_abort_token(cancel.clone()));
    let tools = f.tools;
    let running = tokio::spawn(async move {
        execute_tool_from_registry(&tools, "word_control", request, context).await
    });
    assert_eq!(receive(&mut socket).await["type"], "command");
    cancel.cancel();
    assert!(running.await.unwrap().is_err());
    assert!(f.transport.documents(&office_owner(&f.context).unwrap()).is_empty());
}

#[tokio::test]
async fn cc_control_16_factory_registers_word_with_shared_gateway_authority() {
    let runtime = super::computer_control::NativeControlRuntime::start(Default::default()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let config = crate::config::Config {
        workspace_dir: root.path().into(),
        config_path: root.path().join("config.toml"),
        ..Default::default()
    };
    let context =
        super::ToolRuntimeContext::new("registry", Some(root.path().to_string_lossy().into()))
            .with_test_config(config);
    let tools = super::toolset::execution_tools_for_context(&context);
    assert_eq!(tools.iter().filter(|tool| tool.name() == "word_control").count(), 1);
    let tool = tools.iter().find(|tool| tool.name() == "word_control").unwrap();
    assert!(tool.spec().destructive);
    assert!(!tool.spec().concurrency_safe);
    assert!(
        execute_tool_from_registry(
            &tools,
            "word_control",
            json!({"operation":"discover"}),
            context.tool_use_context()
        )
        .await
        .is_err()
    );
    assert!(Arc::ptr_eq(
        &runtime,
        &super::computer_control::NativeControlRuntime::shared().unwrap()
    ));
}

#[tokio::test]
async fn cc_control_16_malformed_readback_is_failure_and_closes_connection() {
    let f = Fixture::new().await;
    let mut socket = f.pair("book", &["word_read_document"]).await;
    let request = f.request("book", read());
    let responder = tokio::spawn(async move {
        let command = receive(&mut socket).await;
        socket.send(Message::text(json!({"type":"result","sequence":1,"request_id":command["request_id"],"value":{"sheet":"Sheet1","range":"A1:B2","address":"Sheet1!A1:B2","values":[[42]],"formulas":[],"number_format":[]}}).to_string())).await.unwrap();
        let _ = socket.next().await;
    });
    assert!(f.call(request).await.unwrap_err().to_string().contains("invalid Word readback"));
    responder.await.unwrap();
    assert!(f.transport.documents(&office_owner(&f.context).unwrap()).is_empty());
}

struct DocumentApproval(Arc<parking_lot::Mutex<Vec<(String, Value)>>>);
#[async_trait::async_trait]
impl crate::hooks::HookHandler for DocumentApproval {
    fn name(&self) -> &str {
        "cc16-approval-boundary"
    }
    async fn permission_request(&self, name: &str, input: &Value) -> anyhow::Result<Option<bool>> {
        self.0.lock().push((name.into(), input.clone()));
        Ok(Some(true))
    }
}
#[tokio::test]
async fn cc_control_16_document_approval_is_scoped_and_explicit_deny_is_preserved() {
    let f = Fixture::new().await;
    let _socket = f.pair("book", &["word_read_document"]).await;
    let request = f.request("book", read());
    f.runtime
        .policy
        .write()
        .set_grant(&local_user(), GrantScope::WordDocument("book".into()), None)
        .unwrap();
    let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let mut hooks = crate::hooks::HookRunner::new();
    hooks.register(Box::new(DocumentApproval(seen.clone())));
    let context = Arc::new(f.context.as_ref().clone().with_hook_runner(Arc::new(hooks)));
    super::scope_tool_use_context(context.clone(), f.tools[0].check_permissions(&request))
        .await
        .unwrap();
    assert_eq!(seen.lock().len(), 1);
    assert_eq!(seen.lock()[0].0, "word_control.document");
    assert_eq!(seen.lock()[0].1["document_id"], "book");
    assert_eq!(
        f.runtime.policy.read().config().users[&local_user()].word_documents["book"],
        GrantDecision::Allow
    );
    f.runtime
        .policy
        .write()
        .set_grant(
            &local_user(),
            GrantScope::WordDocument("book".into()),
            Some(GrantDecision::Deny),
        )
        .unwrap();
    assert!(
        super::scope_tool_use_context(context, f.tools[0].check_permissions(&request))
            .await
            .is_err()
    );
    assert_eq!(seen.lock().len(), 1);
}
