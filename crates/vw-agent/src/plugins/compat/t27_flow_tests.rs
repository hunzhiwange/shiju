//! Replace only provider networking and user answers; keep source checks, question store,
//! callback listener, HTTP token exchange and their cancellation lifetimes real.
use super::{
    mcp_elicitation::FormOwner, mcp_oauth_discovery::bind, mcp_oauth_flow::authorize_with,
};
use crate::question;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;

async fn run(scenario: &'static str) {
    let temp = tempfile::tempdir().unwrap();
    let options = super::t17_tests::options(temp.path());
    let dcr = scenario.starts_with("dcr");
    let cimd = scenario.starts_with("cimd");
    let mut config = json!({"url":"https://mcp.example/mcp","oauth":{"client_id":"public-fixture"},"scopes":["read"]});
    if dcr {
        config["oauth"] = json!({});
    }
    if cimd {
        config["extensions"] = json!({"com.openai":{"auth":{"type":"oauth","client":{"mode":"cimd","clientId":"https://client.example/client.json"}}}});
    }
    if scenario == "discovered-scopes" {
        config.as_object_mut().unwrap().remove("scopes");
    }
    if scenario == "empty-scopes" {
        config["scopes"] = json!([]);
    }
    let root = super::t17_tests::plugin(&options, "flow", config);
    let manifest = root.join(".codex-plugin/plugin.json");
    let original = std::fs::read(&manifest).unwrap();
    let mut servers = Vec::new();
    crate::plugins::mcp::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let agent_client_protocol::McpServer::Http(server) = servers.remove(0) else {
        panic!("http fixture")
    };
    let mut metadata = bind(&server.url, json!({"resource":server.url,"authorization_servers":["https://auth.example"]}), json!({"issuer":"https://auth.example","authorization_endpoint":"https://auth.example/authorize","token_endpoint":"https://auth.example/token"})).unwrap();
    metadata.client_id = if dcr {
        None
    } else if cimd {
        Some("https://client.example/client.json".into())
    } else {
        Some("public-fixture".into())
    };
    metadata.scopes_supported = Some(vec!["discovered.read".into()]);
    metadata.registration_endpoint = Some("https://auth.example/register".into());
    metadata.client_id_metadata_document_supported = scenario == "cimd";
    metadata.token_endpoint_auth_methods_supported = Some(vec!["none".into()]);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token_url =
        url::Url::parse(&format!("http://{}/token", listener.local_addr().unwrap())).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let provider = tokio::spawn(async move {
        for index in 0..if scenario == "dcr" { 2 } else { 1 } {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let (headers, length) = loop {
                bytes.push(stream.read_u8().await.unwrap());
                if bytes.ends_with(b"\r\n\r\n") {
                    let headers = String::from_utf8(bytes.clone()).unwrap();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (headers, length);
                }
                assert!(bytes.len() < 8192);
            };
            let mut body = vec![0; length];
            stream.read_exact(&mut body).await.unwrap();
            let body = String::from_utf8(body).unwrap();
            let response = if dcr && index == 0 {
                let request: Value = serde_json::from_str(&body).unwrap();
                let mut response = json!({"client_id":"registered-fixture","token_endpoint_auth_method":"none","redirect_uris":request["redirect_uris"]});
                if scenario == "dcr-secret" {
                    response["client_secret"] = json!("synthetic-secret");
                }
                response.to_string()
            } else {
                String::from(r#"{"access_token":"synthetic-fixture-token","token_type":"Bearer"}"#)
            };
            tx.send((headers, body)).unwrap();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).as_bytes()).await.unwrap();
        }
    });
    let session = format!("t27-flow-{scenario}");
    let owner = FormOwner {
        session: session.clone(),
        tenant: crate::session::ui_store::current_tenant_scope(),
    };
    let cancelled = CancellationToken::new();
    let task_cancelled = cancelled.clone();
    let task = tokio::spawn(async move {
        authorize_with(
            &server,
            owner,
            task_cancelled,
            Duration::from_secs(2),
            std::future::ready(Ok(metadata)),
            |endpoint, body, content_type| {
                let mut token_url = token_url.clone();
                async move {
                    assert!(matches!(
                        endpoint.as_str(),
                        "https://auth.example/token" | "https://auth.example/register"
                    ));
                    token_url.set_path(endpoint.path());
                    // The only substituted boundary is the external provider address.
                    let client = reqwest_012::Client::builder()
                        .no_proxy()
                        .redirect(reqwest_012::redirect::Policy::none())
                        .build()?;
                    super::mcp_oauth_flow::send_provider_request(
                        client,
                        token_url,
                        body,
                        content_type,
                    )
                    .await
                }
            },
        )
        .await
    });
    if matches!(scenario, "cimd-unsupported" | "dcr-secret") {
        let message = format!("{:#}", task.await.unwrap().unwrap_err());
        assert!(
            message.contains(if cimd { "advertise CIMD" } else { "without a secret" }),
            "{message}"
        );
        assert!(!question::list().iter().any(|q| q.session_id == session));
        if dcr {
            let (headers, _) = rx.recv().await.unwrap();
            assert!(headers.starts_with("POST /register "));
        }
        assert!(rx.try_recv().is_err());
        provider.abort();
        let _ = provider.await;
        assert_eq!(std::fs::read(manifest).unwrap(), original);
        return;
    }
    let pending = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(pending) = question::list().into_iter().find(|q| q.session_id == session) {
                break pending;
            }
            assert!(!task.is_finished(), "authorization must reach its real user question");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let authorization =
        url::Url::parse(pending.questions[0].question.lines().last().unwrap()).unwrap();
    let query: HashMap<_, _> =
        authorization.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    assert_eq!(query["resource"], "https://mcp.example/mcp");
    if scenario == "empty-scopes" {
        assert!(!query.contains_key("scope"));
    } else {
        assert_eq!(
            query["scope"],
            if scenario == "discovered-scopes" { "discovered.read" } else { "read" }
        );
    }
    let mut callback = url::Url::parse(&query["redirect_uri"]).unwrap();
    let mut browser = None;
    match scenario {
        "deny" => {
            question::reply(question::ReplyInput {
                request_id: pending.id,
                answers: vec![vec!["decline".into()]],
            });
        }
        "cancel" => cancelled.cancel(),
        "timeout" => {}
        _ => {
            if scenario == "changed-source" {
                let mut changed: Value = serde_json::from_slice(&original).unwrap();
                changed["mcpServers"]["shared"]["scopes"] = json!(["write"]);
                std::fs::write(&manifest, serde_json::to_vec(&changed).unwrap()).unwrap();
            }
            let state = if scenario == "forged-state" { "forged" } else { &query["state"] };
            callback
                .query_pairs_mut()
                .append_pair("state", state)
                .append_pair("code", "synthetic-code");
            if scenario == "wrong-issuer" {
                callback.query_pairs_mut().append_pair("iss", "https://other.example");
            }
            question::reply(question::ReplyInput {
                request_id: pending.id,
                answers: vec![vec!["accept".into()]],
            });
            browser = Some(tokio::spawn(async move {
                reqwest_012::Client::builder()
                    .no_proxy()
                    .build()
                    .unwrap()
                    .get(callback)
                    .send()
                    .await
            }));
        }
    }
    let result = tokio::time::timeout(Duration::from_secs(4), task).await.unwrap().unwrap();
    if let Some(browser) = browser {
        let _ = browser.await.unwrap();
    }
    if matches!(scenario, "success" | "dcr" | "cimd" | "discovered-scopes" | "empty-scopes") {
        assert_eq!(result.unwrap(), "synthetic-fixture-token");
        if dcr {
            let (headers, body) = rx.recv().await.unwrap();
            assert!(headers.starts_with("POST /register HTTP/1.1"));
            let registration: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(registration["client_name"], "VibeWindow");
            assert_eq!(registration["redirect_uris"], json!([query["redirect_uri"]]));
            assert_eq!(registration["token_endpoint_auth_method"], "none");
            assert_eq!(query["client_id"], "registered-fixture");
        }
        let (headers, body) = rx.recv().await.unwrap();
        assert!(headers.starts_with("POST /token HTTP/1.1"));
        assert!(!headers.to_ascii_lowercase().contains("authorization:"));
        let form: HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes())
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(form["grant_type"], "authorization_code");
        assert_eq!(form["client_id"], query["client_id"]);
        assert_eq!(form["code"], "synthetic-code");
        assert_eq!(form["resource"], query["resource"]);
        assert_eq!(form["redirect_uri"], query["redirect_uri"]);
        assert_eq!(
            URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes())),
            query["code_challenge"]
        );
        provider.await.unwrap();
    } else {
        let message = format!("{:#}", result.unwrap_err());
        let expected = match scenario {
            "deny" => "denied",
            "cancel" => "cancelled",
            "timeout" => "timed out",
            "forged-state" => "state mismatch",
            "wrong-issuer" => "issuer mismatch",
            "changed-source" => "changed",
            _ => unreachable!(),
        };
        assert!(message.contains(expected), "{scenario}: {message}");
        assert!(rx.try_recv().is_err(), "rejected flows must not exchange credentials");
        provider.abort();
        let _ = provider.await;
    }
    assert!(!question::list().iter().any(|q| q.session_id == session));
    let callback_address = url::Url::parse(&query["redirect_uri"]).unwrap();
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", callback_address.port().unwrap()))
            .await
            .is_err(),
        "terminal flows must close their loopback listener"
    );
    if scenario != "changed-source" {
        assert_eq!(std::fs::read(manifest).unwrap(), original);
    }
}
#[tokio::test]
async fn codex_plugin_compat_t27_callback_exchanges_matching_pkce() {
    run("success").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_denial_never_exchanges_token() {
    run("deny").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_cancellation_cleans_question() {
    run("cancel").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_timeout_closes_callback() {
    run("timeout").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_forged_callback_never_exchanges_token() {
    run("forged-state").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_source_change_never_exchanges_token() {
    run("changed-source").await;
}

#[tokio::test]
async fn codex_plugin_compat_t27_url_stdio_completes_exact_request() {
    use super::mcp_connections::McpSessionConnections;
    let pool = std::sync::Arc::new(McpSessionConnections::for_form_owner(FormOwner {
        session: "t27-stdio-url".into(),
        tenant: crate::session::ui_store::current_tenant_scope(),
    }));
    let task_pool = pool.clone();
    let server = serde_json::from_value(json!({"name":"url-fixture","command":"python3","args":[format!("{}/testdata/plugin_compat/t27/server.py",env!("CARGO_MANIFEST_DIR"))],"env":[]})).unwrap();
    let task = tokio::spawn(async move {
        task_pool
            .with_peer(&server, None, |peer| {
                Box::pin(async move {
                    let result = peer
                        .call_tool(
                            serde_json::from_value(json!({"name":"authorize","arguments":{}}))
                                .unwrap(),
                        )
                        .await?;
                    Ok(serde_json::to_value(result)?)
                })
            })
            .await
            .unwrap()
    });
    let pending = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(pending) =
                question::list().into_iter().find(|q| q.session_id == "t27-stdio-url")
            {
                break pending;
            }
            assert!(!task.is_finished(), "URL request must reach session interaction");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(pending.questions[0].question.contains("url-completion-91"));
    question::reply(question::ReplyInput {
        request_id: pending.id,
        answers: vec![vec!["accept".into()]],
    });
    let result = tokio::time::timeout(Duration::from_secs(10), task).await.unwrap().unwrap();
    pool.shutdown();
    let response: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(response["id"], 91);
    assert_eq!(response["result"]["action"], "accept");
    assert!(response["result"].get("content").is_none_or(Value::is_null));
}

#[tokio::test]
async fn codex_plugin_compat_t27_dcr_registers_public_loopback_client() {
    run("dcr").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_dcr_rejects_secret_client() {
    run("dcr-secret").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_cimd_uses_only_configured_public_identity() {
    run("cimd").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_cimd_requires_server_advertisement() {
    run("cimd-unsupported").await;
}

#[tokio::test]
async fn codex_plugin_compat_t27_scopes_fall_back_to_discovery() {
    run("discovered-scopes").await;
}
#[tokio::test]
async fn codex_plugin_compat_t27_explicit_empty_scopes_never_expand() {
    run("empty-scopes").await;
}

#[tokio::test]
async fn codex_plugin_compat_t27_callback_rejects_wrong_issuer() {
    run("wrong-issuer").await;
}
