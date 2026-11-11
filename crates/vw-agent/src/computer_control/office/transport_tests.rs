use super::*;
use rustls_pki_types::{ServerName, pem::PemObject};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

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

async fn host() -> OfficeTransport {
    OfficeTransport::bind(certificates().path(), "https://localhost:3000").await.unwrap()
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

fn grant(host: &OfficeTransport) -> String {
    host.approve_pairing(
        ControlSessionId("owner".into()),
        OfficeSurface::Excel,
        ControlDocumentId("book".into()),
    )
    .unwrap()
}

async fn pair(host: &OfficeTransport, token: &str) -> Client {
    let mut socket = client(host, "https://localhost:3000").await.unwrap();
    socket.send(Message::text(serde_json::json!({"type":"pair", "token":token, "surface":"excel", "document_id":"book", "title":"Untitled", "capabilities":["excel_read_range"]}).to_string())).await.unwrap();
    socket
}

async fn receive(socket: &mut Client) -> serde_json::Value {
    let message = tokio::time::timeout(Duration::from_secs(3), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(message.to_text().unwrap()).unwrap()
}

async fn rejected(socket: &mut Client) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(3), socket.next()).await.unwrap(),
        None | Some(Err(_)) | Some(Ok(Message::Close(_)))
    )
}

#[tokio::test]
async fn cc_control_12_pairs_once_and_cleans_up_on_disconnect() {
    let host = host().await;
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    assert_eq!(receive(&mut socket).await["type"], "paired");
    assert_eq!(host.documents(&ControlSessionId("owner".into())).len(), 1);
    let mut replay = pair(&host, &token).await;
    assert!(rejected(&mut replay).await);
    socket.close(None).await.unwrap();
    for _ in 0..100 {
        if host.documents(&ControlSessionId("owner".into())).is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(host.documents(&ControlSessionId("owner".into())).is_empty());
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_rejects_wrong_origin_and_untrusted_tls() {
    let host = host().await;
    assert!(client(&host, "https://localhost:3001").await.is_err());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(rustls::RootCertStore::empty())
    .with_no_client_auth();
    let tcp = TcpStream::connect(host.address()).await.unwrap();
    assert!(
        tokio_rustls::TlsConnector::from(Arc::new(config))
            .connect(ServerName::try_from("localhost").unwrap(), tcp)
            .await
            .is_err()
    );
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_expired_pairing_and_connection_are_rejected() {
    let host = host().await;
    let token = grant(&host);
    host.state.lock().pairings.get_mut(&token).unwrap().expires = Instant::now();
    assert!(rejected(&mut pair(&host, &token).await).await);
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    receive(&mut socket).await;
    let handle = host.documents(&ControlSessionId("owner".into()))[0].handle.clone();
    host.state.lock().connections.get_mut(&handle.generation).unwrap().expires = Instant::now();
    socket.send(Message::text(r#"{"type":"heartbeat","sequence":1}"#)).await.unwrap();
    assert!(rejected(&mut socket).await);
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_surface_document_and_sequence_cannot_be_substituted() {
    let host = host().await;
    for (surface, document) in [("word", "book"), ("excel", "other")] {
        let token = grant(&host);
        let mut socket = client(&host, "https://localhost:3000").await.unwrap();
        socket.send(Message::text(serde_json::json!({"type":"pair","token":token,"surface":surface,"document_id":document,"title":"Untitled","capabilities":[]}).to_string())).await.unwrap();
        assert!(rejected(&mut socket).await);
    }
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    receive(&mut socket).await;
    socket.send(Message::text(r#"{"type":"heartbeat","sequence":1}"#)).await.unwrap();
    assert_eq!(receive(&mut socket).await["type"], "heartbeat");
    socket.send(Message::text(r#"{"type":"heartbeat","sequence":1}"#)).await.unwrap();
    assert!(rejected(&mut socket).await);
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_commands_require_owner_target_and_matching_result() {
    let host = Arc::new(host().await);
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    receive(&mut socket).await;
    let owner = ControlSessionId("owner".into());
    let handle = host.documents(&owner)[0].handle.clone();
    let target =
        ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId("book".into()) };
    let action = ComputerControlAction::Excel {
        command: vw_api_types::computer_control::ExcelCommand::ReadRange {
            sheet: "Sheet1".into(),
            range: "A1".into(),
        },
    };
    assert!(
        host.execute(&ControlSessionId("other".into()), &handle, &target, &action).await.is_err()
    );
    let sender = host.clone();
    let job = tokio::spawn(async move { sender.execute(&owner, &handle, &target, &action).await });
    let command = receive(&mut socket).await;
    assert_eq!(command["type"], "command");
    socket.send(Message::text(serde_json::json!({"type":"result","sequence":1,"request_id":command["request_id"],"value":{"values":[[42]]}}).to_string())).await.unwrap();
    assert_eq!(job.await.unwrap().unwrap()["values"][0][0], 42);
    receive(&mut socket).await;
    socket.send(Message::text(serde_json::json!({"type":"result","sequence":2,"request_id":command["request_id"],"value":null}).to_string())).await.unwrap();
    assert!(rejected(&mut socket).await);
    Arc::try_unwrap(host).ok().unwrap().shutdown().await;
}

#[tokio::test]
async fn cc_control_12_unknown_oversized_messages_and_revoked_connections_fail_closed() {
    let host = host().await;
    for message in [r#"{"type":"eval","sequence":1}"#.to_string(), "x".repeat(MAX_BYTES + 1)] {
        let token = grant(&host);
        let mut socket = pair(&host, &token).await;
        receive(&mut socket).await;
        socket.send(Message::text(message)).await.unwrap();
        assert!(rejected(&mut socket).await);
    }
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    receive(&mut socket).await;
    host.revoke(&ControlSessionId("owner".into()));
    assert!(rejected(&mut socket).await);
    assert!(host.documents(&ControlSessionId("owner".into())).is_empty());
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_rejects_non_loopback_origin_and_insecure_key_permissions() {
    assert!(OfficeTransport::bind(certificates().path(), "https://example.com").await.is_err());
    let dir = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    for file in ["cert.pem", "key.pem"] {
        std::fs::copy(certificates().path().join(file), dir.path().join(file)).unwrap();
        std::fs::set_permissions(dir.path().join(file), std::fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    std::fs::set_permissions(dir.path().join("key.pem"), std::fs::Permissions::from_mode(0o644))
        .unwrap();
    assert!(OfficeTransport::bind(dir.path(), "https://localhost:3000").await.is_err());
}

#[tokio::test]
async fn cc_control_12_node_webview_boundary_uses_real_verified_wss_and_rust_commands() {
    let host = host().await;
    let module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/shiju-computer-control/office/shared/connection.mjs")
        .canonicalize()
        .unwrap();
    let script = r#"
const { connectOffice } = await import(process.env.OFFICE_MODULE);
// Node has no browser origin. Model only that WebView boundary; retain real TLS.
class WebViewSocket extends WebSocket { constructor(url) { super(url, {headers:{Origin:'https://localhost:3000'}}); } }
const connection = connectOffice({endpoint:process.env.OFFICE_ENDPOINT, token:process.env.OFFICE_PAIR_TOKEN, WebSocketImpl:WebViewSocket, surface:'excel',documentId:'book',title:'Temporary test workbook',capabilities:['excel_read_range'],execute:async action => { if(action.command.kind !== 'read_range') throw Error('unexpected command'); setTimeout(()=>connection.close(), 500); return {values:[[42]]}; }});
await connection.ready;
"#;
    let mut child = tokio::process::Command::new("node")
        .args(["--input-type=module", "-e", script])
        .env("NODE_EXTRA_CA_CERTS", certificates().path().join("cert.pem"))
        .env("OFFICE_MODULE", url::Url::from_file_path(module).unwrap().as_str())
        .env("OFFICE_ENDPOINT", format!("wss://localhost:{}/office", host.address().port()))
        .env("OFFICE_PAIR_TOKEN", grant(&host))
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let owner = ControlSessionId("owner".into());
    let handle = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(session) = host.documents(&owner).first() {
                break session.handle.clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let target =
        ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId("book".into()) };
    let action = ComputerControlAction::Excel {
        command: vw_api_types::computer_control::ExcelCommand::ReadRange {
            sheet: "Sheet1".into(),
            range: "A1".into(),
        },
    };
    assert_eq!(host.execute(&owner, &handle, &target, &action).await.unwrap()["values"][0][0], 42);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    assert!(host.documents(&owner).is_empty());
    host.shutdown().await;
}

#[tokio::test]
async fn cc_control_12_command_timeout_invalidates_connection_and_pending_result() {
    let host = Arc::new(host().await);
    let token = grant(&host);
    let mut socket = pair(&host, &token).await;
    receive(&mut socket).await;
    let owner = ControlSessionId("owner".into());
    let handle = host.documents(&owner)[0].handle.clone();
    let sender = host.clone();
    let job = tokio::spawn(async move {
        let target =
            ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId("book".into()) };
        let action = ComputerControlAction::Excel {
            command: vw_api_types::computer_control::ExcelCommand::ReadRange {
                sheet: "Sheet1".into(),
                range: "A1".into(),
            },
        };
        sender.execute(&owner, &handle, &target, &action).await
    });
    assert_eq!(receive(&mut socket).await["type"], "command");
    tokio::time::pause();
    tokio::time::advance(TIMEOUT + Duration::from_secs(1)).await;
    assert!(job.await.unwrap().is_err());
    assert!(host.documents(&ControlSessionId("owner".into())).is_empty());
    Arc::try_unwrap(host).ok().unwrap().shutdown().await;
}
