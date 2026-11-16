use super::word_control::*;
use super::excel_control::office_owner;

fn local_user() -> String {
    format!("uid:{}", unsafe { libc::geteuid() })
}

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
use vw_api_computer_control::*;
use config_computer_control::GrantDecision;

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
            .control_policy()
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
        .control_policy()
        .write()
        .set_grant(&local_user(), GrantScope::WordDocument("book".into()), None)
        .unwrap();
    let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let mut hooks = crate::hooks::new_hook_runner();
    hooks.register(Box::new(DocumentApproval(seen.clone())));
    let context = Arc::new(f.context.as_ref().clone().with_hook_runner(Arc::new(hooks)));
    super::scope_tool_use_context(context.clone(), f.tools[0].check_permissions(&request))
        .await
        .unwrap();
    assert_eq!(seen.lock().len(), 1);
    assert_eq!(seen.lock()[0].0, "word_control.document");
    assert_eq!(seen.lock()[0].1["document_id"], "book");
    assert_eq!(
        f.runtime.control_policy().read().config().users[&local_user()].word_documents["book"],
        GrantDecision::Allow
    );
    f.runtime
        .control_policy()
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
