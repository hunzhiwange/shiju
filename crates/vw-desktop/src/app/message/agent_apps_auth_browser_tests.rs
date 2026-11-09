use super::{
    browser::{LinkAction, edit_paste, read_preference, save_preference, submit_paste},
    flow::{Update, run},
};
use crate::app::{
    components::system_settings_auth::login_modal_tests::{Exchange, gateway, next},
    state::agent_apps_auth::{LoginPhase, LoginTarget, Scope, State, now},
};
use iced::futures::channel::mpsc;
use serde_json::json;
use std::cell::Cell;

const FLOW: &str = "743b63aa-a8b8-4c1d-8c99-40324b431d92";
const NEXT: &str = "7cb10748-4eb9-4424-a67d-d4ec08509645";
const URL: &str = "https://claude.example/authorize?state=fixture";
const PASTE: &str = "fixture-code#fixture";

#[tokio::test]
async fn auth6_ui_browser_commits_expected_scoped_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth-browser.json");
    assert!(read_preference(&path), "a new desktop opens the browser");
    let url = reqwest::Url::parse(URL).unwrap();
    let opened = Cell::new(false);
    LinkAction::for_preference(!read_preference(&path))
        .deliver_using(
            &url,
            |link| {
                opened.set(link == URL);
                true
            },
            |_| panic!("unexpected copy"),
        )
        .unwrap();
    assert!(opened.get());

    save_preference(&path, false).unwrap();
    assert!(!read_preference(&path), "reopening the desktop must preserve copy-only mode");

    let deadline = now().0 + 30_000;
    let (client, connection, server) = gateway(
        "claude_official",
        vec![
            Exchange::claude_start(FLOW, "copy_link", URL, deadline),
            Exchange::paste(FLOW, PASTE, true),
            Exchange::poll(json!({
                "state": "approved", "flow_id": FLOW, "expires_at": deadline,
                "account": {"account_id": "opaque", "service": "claude_official",
                    "label": "合成账号", "credential_state": "ready", "is_default": false},
            })),
        ],
    );
    let mut state = State::with_browser_preference(read_preference(&path));
    state.select_endpoint(client.endpoint());
    let target = LoginTarget::Claude;
    let (generation, cancelled, pastes) = state.begin_login(target);
    let (sender, mut updates) = mpsc::channel(4);
    let task = tokio::spawn(run(
        client,
        connection,
        target,
        state.copy_login_link,
        cancelled,
        pastes,
        sender,
    ));
    let Update::Started(challenge) = next(&mut updates).await else {
        panic!("expected Claude browser challenge");
    };
    assert!(challenge.can_paste);
    let copied = Cell::new(false);
    LinkAction::for_preference(state.copy_login_link)
        .deliver_using(
            &challenge.verification_url,
            |_| panic!("unexpected open"),
            |link| {
                copied.set(link == URL);
                true
            },
        )
        .unwrap();
    assert!(copied.get());

    state.login_mut(generation).unwrap().phase = LoginPhase::Pending(challenge);
    edit_paste(&mut state, generation, PASTE.to_owned());
    submit_paste(&mut state, generation).unwrap();
    assert!(state.login_mut(generation).unwrap().paste_input.is_empty());
    assert!(submit_paste(&mut state, generation).is_err(), "in-flight paste cannot be sent twice");
    assert!(matches!(next(&mut updates).await, Update::PasteFinished(Ok(()))));
    assert!(matches!(next(&mut updates).await, Update::Approved));
    task.await.unwrap();
    server.join().unwrap();
    save_preference(&path, true).unwrap();
    assert!(read_preference(&path));
}

#[tokio::test]
async fn auth6_ui_browser_rejects_invalid_or_stale_input() {
    let url = reqwest::Url::parse(URL).unwrap();
    assert_eq!(
        LinkAction::Open.deliver_using(&url, |_| false, |_| panic!("unexpected copy")),
        Err("无法打开授权浏览器")
    );
    assert_eq!(
        LinkAction::Copy.deliver_using(&url, |_| panic!("unexpected open"), |_| false),
        Err("无法复制授权链接")
    );

    let deadline = now().0 + 30_000;
    let (client, connection, server) = gateway(
        "claude_official",
        vec![
            Exchange::claude_start(FLOW, "browser", URL, deadline),
            Exchange::paste(FLOW, "different-code#different-state", false),
            Exchange::cancel(FLOW),
            Exchange::claude_start(
                NEXT,
                "browser",
                "https://user:password@claude.example/authorize",
                deadline,
            ),
            Exchange::cancel(NEXT),
        ],
    );
    let mut state = State::default();
    state.select_endpoint(client.endpoint());
    let (old, _, _) = state.begin_login(LoginTarget::Claude);
    let (generation, cancelled, pastes) = state.begin_login(LoginTarget::Claude);
    let (sender, mut updates) = mpsc::channel(4);
    let task = tokio::spawn(run(
        client.clone(),
        connection.clone(),
        LoginTarget::Claude,
        false,
        cancelled,
        pastes,
        sender,
    ));
    let Update::Started(challenge) = next(&mut updates).await else { panic!("expected challenge") };
    state.login_mut(generation).unwrap().phase = LoginPhase::Pending(challenge);

    edit_paste(&mut state, old, PASTE.to_owned());
    assert!(
        submit_paste(&mut state, old).is_err(),
        "a replaced login must reject stale paste events"
    );
    assert!(submit_paste(&mut state, generation).is_err(), "empty current paste must be rejected");
    edit_paste(&mut state, generation, "different-code#different-state".into());
    submit_paste(&mut state, generation).unwrap();
    assert!(matches!(
        next(&mut updates).await,
        Update::PasteFinished(Err("执行主机拒绝了回填，请检查授权码或回调地址"))
    ));
    let different =
        Scope { gateway: "http://127.0.0.1:1".into(), ..Scope::from(client.endpoint()) };
    state.select(different);
    assert!(
        submit_paste(&mut state, generation).is_err(),
        "scope switch discards the old paste route"
    );
    task.await.unwrap();

    state.select_endpoint(client.endpoint());
    let (_, cancelled, pastes) = state.begin_login(LoginTarget::Claude);
    let (sender, mut updates) = mpsc::channel(4);
    let task = tokio::spawn(run(
        client,
        connection,
        LoginTarget::Claude,
        false,
        cancelled,
        pastes,
        sender,
    ));
    assert!(matches!(next(&mut updates).await, Update::Failed("网关返回了无效的授权链接")));
    task.await.unwrap();
    server.join().unwrap();
}
