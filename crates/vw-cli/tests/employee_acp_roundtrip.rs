#![cfg(all(unix, feature = "employee-acp-test-support", debug_assertions))]

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    net::SocketAddr,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use uuid::Uuid;
use vw_api_transport::TenantScope;

const BINARY: &str = env!("CARGO_BIN_EXE_shiju");
const FIXTURE_ADDRESS: &str = "SHIJU_EMPLOYEE_ACP_FIXTURE_ADDRESS";
const NEGATIVE_CONTROL: &str = "SHIJU_EMPLOYEE_ACP_WRONG_ACCOUNT";
const AT: &str = "2026-11-21T00:00:00Z";
const BASE: &str = "/sj/agent/api/v1/";
const PRIVATE: Target = Target {
    collection: "assistants",
    id: "private-worker",
    kind: "private",
    product_kind: "PRIVATE_DIRECT",
};
const SHARED: Target = Target {
    collection: "employees",
    id: "shared-worker",
    kind: "shared",
    product_kind: "SHARED_DIRECT",
};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    collection: &'static str,
    id: &'static str,
    kind: &'static str,
    product_kind: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Account {
    user: &'static str,
    pairing: &'static str,
    login: &'static str,
}

const ALICE: Account = Account { user: "101", pairing: "fixture-pair-a", login: "fixture-login-a" };
const BOB: Account = Account { user: "102", pairing: "fixture-pair-b", login: "fixture-login-b" };

struct Turn {
    id: Uuid,
    message: Uuid,
    reply: Uuid,
    text: String,
    answer: String,
    key: String,
    status: &'static str,
}

impl Turn {
    fn user_message(&self) -> Value {
        json!({"message_id":self.message,"sender":"user","body":self.text,"created_at":AT})
    }

    fn reply_message(&self) -> Value {
        json!({"message_id":self.reply,"sender":"employee","body":self.answer,
            "reply_to_message_id":self.message,"created_at":AT})
    }

    fn status(&self) -> Value {
        json!({"turn_id":self.id,"user_message_id":self.message,"employee_version_id":"fixture-version",
            "status":self.status,"created_at":AT,"updated_at":AT})
    }
}

struct Conversation {
    owner: Account,
    target: Target,
    turns: Vec<Turn>,
}

impl Conversation {
    fn metadata(&self, id: Uuid) -> Value {
        json!({"conversation_id":id,"assistant_id":self.target.id,"kind":self.target.product_kind,
            "active_version_id":"fixture-version","title":"Fixture","status":"ACTIVE","created_at":AT,"updated_at":AT})
    }

    fn history(&self) -> Value {
        let mut messages = Vec::new();
        for turn in &self.turns {
            messages.push(turn.user_message());
            if turn.status == "COMPLETED" {
                messages.push(turn.reply_message());
            }
        }
        json!({"messages":messages,"turns":self.turns.iter().map(Turn::status).collect::<Vec<_>>()})
    }
}

#[derive(Default)]
struct Product {
    conversations: HashMap<Uuid, Conversation>,
    sends: Vec<(Uuid, String, String)>,
    histories: Vec<(Uuid, &'static str)>,
    unexpected: Vec<String>,
    revoked: bool,
    revoked_requests: usize,
}

async fn employee_product(
    State(product): State<Arc<Mutex<Product>>>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let Some(account) = [ALICE, BOB]
        .into_iter()
        .find(|a| parts.headers.get("x-api-key").is_some_and(|key| key == a.login))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if parts.uri.path() == "/iam/api/v1/me" && parts.method == Method::GET {
        let user = if std::env::var_os(NEGATIVE_CONTROL).is_some() && account == ALICE {
            BOB.user
        } else {
            account.user
        };
        return Json(json!({"code":200,"message":"","data":{
            "user_id":user,"company_id":"100000000","account_id":user,"account_name":"fixture","name":"Fixture",
            "source_type":"manual","external_user_id":"","allow_console_login":true,"status":"enabled",
        }})).into_response();
    }
    let route: Vec<_> = parts.uri.path().strip_prefix(BASE).unwrap_or("").split('/').collect();
    let key = parts
        .headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty());
    let bytes = axum::body::to_bytes(body, 16_384).await.unwrap();
    let payload: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let mut product = product.lock().unwrap();
    let response = match (parts.method.clone(), route.as_slice()) {
        (Method::GET, [collection, employee, "conversations"]) => {
            let items: Vec<_> = product
                .conversations
                .iter()
                .filter(|(_, c)| {
                    c.owner == account
                        && c.target.collection == *collection
                        && c.target.id == *employee
                })
                .map(|(&id, c)| c.metadata(id))
                .collect();
            if product.revoked && account == ALICE && *employee == SHARED.id {
                product.revoked_requests += 1;
                return StatusCode::FORBIDDEN.into_response();
            }
            Json(json!({"items":items})).into_response()
        }
        (Method::POST, [collection, employee, "conversations"]) => {
            let Some(target) = [PRIVATE, SHARED]
                .into_iter()
                .find(|t| t.collection == *collection && t.id == *employee)
            else {
                product.unexpected.push(parts.uri.path().to_owned());
                return StatusCode::NOT_FOUND.into_response();
            };
            let id = Uuid::new_v4();
            product
                .conversations
                .insert(id, Conversation { owner: account, target, turns: Vec::new() });
            submit(&mut product, id, key, &payload)
        }
        (method, ["conversations", id, tail @ ..]) => {
            let Ok(id) = Uuid::parse_str(id) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            let Some(c) = product.conversations.get(&id) else {
                return StatusCode::NOT_FOUND.into_response();
            };
            if c.owner != account {
                product.unexpected.push("cross-account conversation request".into());
                return StatusCode::FORBIDDEN.into_response();
            }
            if product.revoked && account == ALICE && c.target == SHARED {
                product.revoked_requests += 1;
                return StatusCode::FORBIDDEN.into_response();
            }
            match (method, tail) {
                (Method::GET, ["messages"]) => {
                    let history = c.history();
                    product.histories.push((id, account.user));
                    Json(history).into_response()
                }
                (Method::POST, ["messages"]) => submit(&mut product, id, key, &payload),
                (Method::POST, ["turns", turn, "cancel"]) => {
                    let turn = Uuid::parse_str(turn).unwrap();
                    let turn = product
                        .conversations
                        .get_mut(&id)
                        .unwrap()
                        .turns
                        .iter_mut()
                        .find(|item| item.id == turn)
                        .unwrap();
                    assert_eq!(turn.status, "RESPONDING");
                    turn.status = "CANCELLED";
                    Json(turn.status()).into_response()
                }
                _ => {
                    product.unexpected.push(parts.uri.path().to_owned());
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }
        _ => {
            product.unexpected.push(parts.uri.path().to_owned());
            StatusCode::NOT_FOUND.into_response()
        }
    };
    response
}

fn submit(product: &mut Product, id: Uuid, key: Option<&str>, payload: &Value) -> Response {
    let (Some(key), Some(text)) = (key, payload["message"].as_str()) else {
        product.unexpected.push("submission missing idempotency key or text".into());
        return StatusCode::BAD_REQUEST.into_response();
    };
    product.sends.push((id, key.into(), text.into()));
    let c = product.conversations.get_mut(&id).unwrap();
    let existing = c.turns.iter().position(|turn| turn.key == key);
    let index = if let Some(index) = existing {
        assert_eq!(c.turns[index].text, text);
        index
    } else {
        let answer = if text == "recall" {
            format!("marker {}", c.turns.first().expect("recall needs history").text)
        } else {
            format!("answer {text}")
        };
        c.turns.push(Turn {
            id: Uuid::new_v4(),
            message: Uuid::new_v4(),
            reply: Uuid::new_v4(),
            text: text.into(),
            answer,
            key: key.into(),
            status: if text == "hold" { "RESPONDING" } else { "COMPLETED" },
        });
        c.turns.len() - 1
    };
    if text == "lost-response" && existing.is_none() {
        // The turn committed, but the caller received no successful submission response.
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let turn = &c.turns[index];
    (
        StatusCode::CREATED,
        Json(json!({"conversation":c.metadata(id),"message":turn.user_message(),
        "turn":turn.status(),"idempotent":existing.is_some()})),
    )
        .into_response()
}

#[tokio::test]
#[ignore = "invoked by the roundtrip in an isolated process"]
async fn seed_and_exec_gateway() {
    let root = std::env::current_dir().unwrap();
    assert_eq!(std::env::var_os("VIBEWINDOW_TEST_ROOT").unwrap(), root.as_os_str());
    let _ = rustls::crypto::ring::default_provider().install_default();
    vw_agent::Config::load_or_init().await.unwrap();
    let tenant = TenantScope::default();
    let keys = [ALICE, BOB].map(|account| config_gateway::GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: gateway_pairing::pairing::hash_skey(account.pairing),
        masked_skey: String::new(),
        name: "fixture".into(),
        expires_at: None,
        user_id: Some(account.user.into()),
    });
    vw_agent::config::replace_gateway_skeys_for_tenant(&tenant, &keys).await.unwrap();
    let mut profile = agent_config::catalog::merged_agent_catalog(&Default::default())
        .remove("acp-employee-acp")
        .unwrap();
    profile.enabled = true;
    vw_agent::config::update_global_for_tenant(
        &tenant,
        json!({"gateway":{"auth_enabled":true},"agents":{"acp-employee-acp":profile}}),
    )
    .await
    .unwrap();
    panic!("exec shiju gateway: {}", gateway_command().exec());
}

fn gateway_command() -> Command {
    let mut command = Command::new(BINARY);
    command.args(["gateway", "--host", "127.0.0.1", "--port", "0"]);
    command
}

struct Gateway {
    child: Child,
    base: String,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = Command::new("/bin/kill").args(["-TERM", &self.child.id().to_string()]).status();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Gateway {
    async fn start(root: &Path, address: SocketAddr, seed: bool, client: &reqwest::Client) -> Self {
        let log = tempfile::NamedTempFile::new_in(root).unwrap();
        let output = OpenOptions::new().append(true).open(log.path()).unwrap();
        let mut command = if seed {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args(["--ignored", "--exact", "seed_and_exec_gateway", "--nocapture"]);
            command
        } else {
            gateway_command()
        };
        let child = command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", root)
            .env("TMPDIR", root)
            .env("VIBEWINDOW_TEST_ROOT", root)
            .env(FIXTURE_ADDRESS, address.to_string())
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
        let mut gateway = Self { child, base: String::new() };
        gateway.base = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let output = fs::read_to_string(log.path()).unwrap();
                assert!(gateway.child.try_wait().unwrap().is_none(), "gateway exited: {output}");
                if let Some(base) = output.lines().find_map(|line| {
                    line.strip_prefix("VibeWindow Gateway listening on ")
                        .filter(|s| s.starts_with("http://127.0.0.1:"))
                }) && client
                    .get(format!("{base}/v1/global/health"))
                    .send()
                    .await
                    .is_ok_and(|response| response.status().is_success())
                {
                    break base.to_owned();
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!("gateway did not start: {}", fs::read_to_string(log.path()).unwrap())
        });
        gateway
    }
}

struct Chat {
    client: reqwest::Client,
    directory: PathBuf,
}

impl Chat {
    fn auth(request: reqwest::RequestBuilder, account: Account) -> reqwest::RequestBuilder {
        request
            .bearer_auth(account.pairing)
            .header("x-user-id", account.user)
            .header("x-vibewindow-shiju-credential", account.login)
    }

    async fn send(
        &self,
        gateway: &Gateway,
        account: Account,
        request_id: Uuid,
        session: Option<&str>,
        target: Option<Target>,
        text: &str,
    ) -> reqwest::Response {
        let mut body = json!({"messages":[{"role":"user","content":text}]});
        if let Some(session) = session {
            body["session_id"] = json!(session);
        }
        if let Some(target) = target {
            body["agent"] = json!("acp-employee-acp");
            body["options"] =
                json!({"employee_target":{"employee_id":target.id,"kind":target.kind}});
        }
        Self::auth(self.client.post(format!("{}/v1/chat/stream", gateway.base)), account)
            .query(&[("directory", self.directory.to_str().unwrap())])
            .header("x-vw-stream-request-id", request_id.to_string())
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn answer(
        &self,
        gateway: &Gateway,
        account: Account,
        session: Option<&str>,
        target: Option<Target>,
        text: &str,
        expected: &str,
    ) -> String {
        let response = self.send(gateway, account, Uuid::new_v4(), session, target, text).await;
        let (session, events) = events(response).await;
        assert!(events.iter().any(|e| e["type"] == "chat.done"), "{events:?}");
        assert!(!events.iter().any(|e| e["type"] == "chat.error"), "{events:?}");
        let answer: String = events
            .iter()
            .filter(|e| e["type"] == "chat.delta")
            .filter_map(|e| e["delta"].as_str())
            .collect();
        assert_eq!(answer, expected);
        session
    }
}

async fn events(response: reqwest::Response) -> (String, Vec<Value>) {
    let status = response.status();
    let session =
        response.headers().get("x-vw-session-id").map(|value| value.to_str().unwrap().to_owned());
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    let events = body
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|data| serde_json::from_str(data.trim()).unwrap())
        .collect();
    (session.expect("chat returned a ShiJu session ID"), events)
}

#[tokio::test]
async fn employee_acp_gateway_roundtrip() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let directory = root_path.join("workspace");
    fs::create_dir(&directory).unwrap();
    let product = Arc::new(Mutex::new(Product::default()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new().fallback(employee_product).with_state(product.clone());
    let fixture = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let chat = Chat {
        client: reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(35))
            .build()
            .unwrap(),
        directory,
    };
    let gateway = Gateway::start(&root_path, address, true, &chat.client).await;

    let private = chat
        .answer(&gateway, ALICE, None, Some(PRIVATE), "private-marker", "answer private-marker")
        .await;
    assert_eq!(
        chat.answer(&gateway, ALICE, Some(&private), None, "recall", "marker private-marker").await,
        private
    );
    let (alice, bob) = tokio::join!(
        chat.answer(&gateway, ALICE, None, Some(SHARED), "alice-marker", "answer alice-marker"),
        chat.answer(&gateway, BOB, None, Some(SHARED), "bob-marker", "answer bob-marker")
    );
    assert_ne!(alice, bob);
    for (account, session, expected) in
        [(ALICE, &alice, "marker alice-marker"), (BOB, &bob, "marker bob-marker")]
    {
        assert_eq!(
            chat.answer(&gateway, account, Some(session), None, "recall", expected).await,
            *session
        );
    }

    for target in
        [Target { id: "other-private-worker", ..PRIVATE }, Target { id: PRIVATE.id, ..SHARED }]
    {
        let wrong_target = chat
            .send(&gateway, ALICE, Uuid::new_v4(), Some(&private), Some(target), "wrong target")
            .await;
        assert_eq!(wrong_target.status(), StatusCode::BAD_REQUEST);
    }
    let cross_account = Chat::auth(
        chat.client
            .get(format!("{}/v1/session/{alice}", gateway.base))
            .query(&[("directory", chat.directory.to_str().unwrap())]),
        BOB,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(cross_account.status(), StatusCode::NOT_FOUND);

    let response =
        chat.send(&gateway, ALICE, Uuid::new_v4(), Some(&private), None, "lost-response").await;
    let (session, failed) = events(response).await;
    assert_eq!(session, private);
    assert!(failed.iter().any(|e| e["type"] == "chat.error"), "{failed:?}");
    assert!(!failed.iter().any(|e| e["type"] == "chat.delta"), "{failed:?}");

    drop(gateway);
    let gateway = Gateway::start(&root_path, address, false, &chat.client).await;
    assert_eq!(
        chat.answer(&gateway, ALICE, Some(&private), None, "lost-response", "answer lost-response")
            .await,
        private
    );
    assert_eq!(
        chat.answer(&gateway, ALICE, Some(&alice), None, "recall", "marker alice-marker").await,
        alice
    );
    assert_eq!(
        chat.answer(&gateway, BOB, Some(&bob), None, "recall", "marker bob-marker").await,
        bob
    );

    let request_id = Uuid::new_v4();
    let held = chat.send(&gateway, ALICE, request_id, Some(&private), None, "hold").await;
    assert_eq!(held.status(), StatusCode::OK);
    tokio::time::timeout(Duration::from_secs(15), async {
        while !product.lock().unwrap().sends.iter().any(|(_, _, text)| text == "hold") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the employee process submitted the held turn");
    let stopped = Chat::auth(
        chat.client.post(format!("{}/v1/chat/stream/{request_id}/stop", gateway.base)),
        ALICE,
    )
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json::<Value>()
    .await
    .unwrap();
    assert_eq!(stopped, json!({"cancelled":true}));
    let (session, cancelled) = events(held).await;
    assert_eq!(session, private);
    assert!(
        cancelled.iter().any(|e| e["type"] == "chat.done" && e["finish_reason"] == "cancelled"),
        "{cancelled:?}"
    );
    assert!(!cancelled.iter().any(|e| e["type"] == "chat.delta"), "{cancelled:?}");
    assert_eq!(
        chat.answer(&gateway, ALICE, Some(&private), None, "after-stop", "answer after-stop").await,
        private
    );

    product.lock().unwrap().revoked = true;
    drop(gateway);
    let gateway = Gateway::start(&root_path, address, false, &chat.client).await;
    let denied =
        chat.send(&gateway, ALICE, Uuid::new_v4(), Some(&alice), None, "after-revoke").await;
    let (_, denied) = events(denied).await;
    assert!(denied.iter().any(|e| e["type"] == "chat.error"), "{denied:?}");
    assert!(!denied.iter().any(|e| e["type"] == "chat.delta"), "{denied:?}");
    assert_eq!(
        chat.answer(&gateway, BOB, Some(&bob), None, "recall", "marker bob-marker").await,
        bob
    );

    let product = product.lock().unwrap();
    assert!(product.unexpected.is_empty(), "{:?}", product.unexpected);
    assert_eq!(product.conversations.len(), 3);
    assert!(product.revoked_requests > 0, "reopen must recheck product authorization");
    let lost: Vec<_> =
        product.sends.iter().filter(|(_, _, text)| text == "lost-response").collect();
    assert_eq!(lost.len(), 2);
    assert_eq!(lost[0], lost[1], "retry must reuse the conversation and submission key");
    let private_turns = &product.conversations[&lost[0].0].turns;
    assert_eq!(private_turns.iter().filter(|t| t.text == "lost-response").count(), 1);
    assert!(private_turns.iter().any(|t| t.text == "hold" && t.status == "CANCELLED"));
    assert!(
        !product
            .sends
            .iter()
            .any(|(_, _, text)| { ["wrong target", "after-revoke"].contains(&text.as_str()) })
    );
    for (&id, c) in &product.conversations {
        assert!(product.histories.contains(&(id, c.owner.user)));
    }
    fixture.abort();
}
