use super::{
    live::{commit::*, write_guard::lock_app},
    mode_state::*,
};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
};
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*, id::TenantScope};
fn tenant() -> TenantScope {
    TenantScope::new("p", "c").unwrap()
}
fn configuration() -> ClientConfiguration {
    ClientConfiguration::saved(AppId::Claude,&json!({"env":{"ANTHROPIC_BASE_URL":"https://fixture.example","ANTHROPIC_API_KEY":"fake-private-secret","ANTHROPIC_MODEL":"fixture","CLAUDE_CODE_DISABLE_ARTIFACT":"1"}}), false).unwrap()
}
fn mode() -> ModeState {
    ModeState {
        mode: Mode::Proxy,
        attached: true,
        proxy_route: Some("route".into()),
        contract: Some(configuration().contract()),
    }
}
#[test]
fn cc04_mode_state_distinguishes_direct_route_and_stack() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(live.path().join("config"), b"user configuration").unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let stack = StackState {
        enabled: true,
        members: vec!["route".into(), "member".into()],
        keys: std::collections::BTreeMap::from([("member-key".into(), "member".into())]),
    };
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    tx.commit(
        &tenant(),
        &mut guard,
        &[],
        PendingTarget {
            pointer: Some("direct".into()),
            mode: Some(mode()),
            stack: Some(stack.clone()),
        },
    )
    .unwrap();
    drop(guard);
    let reopened =
        ProviderTransactions::open_existing(root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap()
            .unwrap();
    let state = reopened.state(AppId::Claude).unwrap();
    let status = observe(&state, &tenant()).unwrap();
    assert_eq!(status.direct_provider.as_deref(), Some("direct"));
    assert_eq!(status.in_use_provider.as_deref(), Some("route"));
    assert!(status.stack.active);
    assert!(status.attached);
    assert!(status.has_contract);
    assert_eq!(state.stack, stack);
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    tx.commit(
        &tenant(),
        &mut guard,
        &[],
        PendingTarget {
            mode: Some(ModeState { mode: Mode::Direct, attached: false, ..mode() }),
            ..Default::default()
        },
    )
    .unwrap();
    let state = tx.state(AppId::Claude).unwrap();
    let status = observe(&state, &tenant()).unwrap();
    assert_eq!(status.in_use_provider.as_deref(), Some("direct"));
    assert!(!status.stack.active);
    assert!(status.stack.enabled);
    assert_eq!(state.stack, stack);
    assert_eq!(fs::read(live.path().join("config")).unwrap(), b"user configuration");
}
#[test]
fn cc04_mode_state_equal_contract_skips_rewrite() {
    let state = AppState { mode: Some(mode()), mode_tenant: Some(tenant()), ..Default::default() };
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Proxy, &configuration()),
        RewriteDecision::SkipRewrite
    );
    let same=ClientConfiguration::saved(AppId::Claude,&json!({"unknown":"ignored","env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ANTHROPIC_MODEL":"fixture","ANTHROPIC_API_KEY":"fake-private-secret","ANTHROPIC_BASE_URL":"https://fixture.example"}}), false).unwrap();
    assert_eq!(rewrite_decision(&state, ProviderMode::Proxy, &same), RewriteDecision::SkipRewrite);
}
#[test]
fn cc04_mode_state_changes_detachment_legacy_and_modes_require_rewrite() {
    let mut state =
        AppState { mode: Some(mode()), mode_tenant: Some(tenant()), ..Default::default() };
    for row in [
        json!({"env":{"ANTHROPIC_API_KEY":"changed"}}),
        json!({"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:4567"}}),
        json!({"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"0"}}),
        json!({"modelPicker":["model"],"env":{}}),
    ] {
        let c = ClientConfiguration::saved(AppId::Claude, &row, false).unwrap();
        assert_eq!(
            rewrite_decision(&state, ProviderMode::Proxy, &c),
            RewriteDecision::RewriteRequired
        );
    }
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Direct, &configuration()),
        RewriteDecision::RewriteRequired
    );
    state.mode.as_mut().unwrap().attached = false;
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Proxy, &configuration()),
        RewriteDecision::RewriteRequired
    );
    state.mode.as_mut().unwrap().attached = true;
    state.mode.as_mut().unwrap().mode = Mode::Direct;
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Direct, &configuration()),
        RewriteDecision::RewriteRequired
    );
    state.mode.as_mut().unwrap().mode = Mode::Proxy;
    state.mode.as_mut().unwrap().contract.as_mut().unwrap().version = 1;
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Proxy, &configuration()),
        RewriteDecision::RewriteRequired
    );
    assert_eq!(
        observe(&state, &TenantScope::new("p", "other").unwrap()),
        Err(ModeStateError::Ownership)
    );
}
#[test]
fn cc04_mode_state_contract_is_canonical_and_application_scoped() {
    let a = ClientConfiguration::ClaudeDesktop {
        profile: json!({"nested":{"a":1,"b":2},"models":["one","two"]}),
    };
    let b = ClientConfiguration::ClaudeDesktop {
        profile: serde_json::from_str(r#"{"models":["one","two"],"nested":{"b":2,"a":1}}"#)
            .unwrap(),
    };
    assert!(a.contract() == b.contract());
    let c = ClientConfiguration::Grok {
        document: json!({"nested":{"a":1,"b":2},"models":["one","two"]}),
    };
    assert!(a.contract() != c.contract());
    let d = ClientConfiguration::ClaudeDesktop {
        profile: json!({"nested":{"a":1,"b":2},"models":["two","one"]}),
    };
    assert!(a.contract() != d.contract());
    assert!(ClientConfiguration::saved(AppId::Pi, &json!({}), false).is_err());
    assert!(ClientConfiguration::saved(AppId::Claude, &json!({"env":[]}), false).is_err());
}
#[test]
fn cc04_mode_state_read_only_does_not_provision_or_touch_native_file() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("native");
    fs::write(&path, b"unchanged").unwrap();
    let before = fs::metadata(&path).unwrap();
    assert!(
        ProviderTransactions::open_existing(root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap()
            .is_none()
    );
    assert!(!root.path().join("provider-live").exists());
    let state = AppState::default();
    let _ = observe(&state, &tenant()).unwrap();
    let _ = rewrite_decision(&state, ProviderMode::Direct, &configuration());
    let after = fs::metadata(&path).unwrap();
    assert_eq!(
        (before.ino(), before.mtime(), before.mtime_nsec()),
        (after.ino(), after.mtime(), after.mtime_nsec())
    );
    assert_eq!(fs::read(path).unwrap(), b"unchanged");
}
#[test]
fn cc04_mode_state_stack_keys_are_permanent_and_tenant_owned() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let mut stack = StackState {
        enabled: true,
        members: vec!["member".into()],
        keys: std::collections::BTreeMap::from([("reserved".into(), "member".into())]),
    };
    tx.commit(
        &tenant(),
        &mut guard,
        &[],
        PendingTarget { stack: Some(stack.clone()), ..Default::default() },
    )
    .unwrap();
    stack.members.clear();
    stack.enabled = false;
    tx.commit(
        &tenant(),
        &mut guard,
        &[],
        PendingTarget { stack: Some(stack.clone()), ..Default::default() },
    )
    .unwrap();
    let before = tx.state(AppId::Claude).unwrap();
    let mut reused = stack.clone();
    reused.keys.insert("reserved".into(), "other".into());
    let mut removed = stack.clone();
    removed.keys.clear();
    let mut duplicate = stack.clone();
    duplicate.members = vec!["member".into(), "member".into()];
    for value in [reused, removed, duplicate] {
        assert!(
            tx.commit(
                &tenant(),
                &mut guard,
                &[],
                PendingTarget { stack: Some(value), ..Default::default() }
            )
            .is_err()
        );
        assert!(tx.state(AppId::Claude).unwrap() == before);
    }
    assert!(
        tx.commit(
            &TenantScope::new("p", "other").unwrap(),
            &mut guard,
            &[],
            PendingTarget { stack: Some(stack), ..Default::default() }
        )
        .is_err()
    );
    assert!(tx.state(AppId::Claude).unwrap() == before);
    assert_eq!(
        observe(&before, &TenantScope::new("p", "other").unwrap()),
        Err(ModeStateError::Ownership)
    );
}
#[test]
fn cc04_mode_state_stack_intent_recovers_with_files_and_rejects_legacy_version() {
    use super::live::{
        commit::{HOOK, Phase},
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    };
    use std::ffi::OsStr;
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(live.path().join("config"), b"before").unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let stack = StackState {
        enabled: true,
        members: vec!["member".into()],
        keys: std::collections::BTreeMap::from([("key".into(), "member".into())]),
    };
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(|tx, app, phase| {
            if phase == Phase::Marked {
                let intent = tx.pending(app)?.unwrap();
                assert_eq!(intent.version, 4);
                assert!(intent.target.stack.is_some());
                return Err(WriteError::Io);
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"after".to_vec());
    let result = tx.commit(
        &tenant(),
        &mut guard,
        &[FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &patch }],
        PendingTarget { stack: Some(stack.clone()), ..Default::default() },
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert!(matches!(result, Err(WriteError::Io)));
    assert_eq!(fs::read(live.path().join("config")).unwrap(), b"before");
    assert!(tx.state(AppId::Claude).unwrap().stack.members.is_empty());
    let path = root.path().join("provider-live/aG9zdA/claude.pending.json");
    let bytes = fs::read(&path).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    legacy["version"] = json!(3);
    fs::write(&path, legacy.to_string()).unwrap();
    assert!(tx.pending(AppId::Claude).is_err());
    fs::write(&path, bytes).unwrap();
    drop(guard);
    let reopened =
        ProviderTransactions::open_existing(root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap()
            .unwrap();
    let results = reopened.recover_on_startup();
    assert_eq!(results.len(), 1);
    assert!(results[0].1.is_ok());
    assert_eq!(reopened.state(AppId::Claude).unwrap().stack, stack);
    assert_eq!(fs::read(live.path().join("config")).unwrap(), b"after");
    assert!(reopened.pending(AppId::Claude).unwrap().is_none());
}
#[test]
fn cc04_mode_state_legacy_state_and_all_client_contracts_are_explicit() {
    let state: AppState =
        serde_json::from_value(json!({"pointer":null,"mode":null,"mode_tenant":null})).unwrap();
    assert!(!state.stack.enabled);
    assert!(state.stack.keys.is_empty());
    let configs = [
        ClientConfiguration::Gemini {
            env: Default::default(),
            config: json!({"model":{"name":"fixture"}}),
        },
        ClientConfiguration::Grok { document: json!({"models":{"default":"fixture"}}) },
        ClientConfiguration::Codex {
            config: json!({"model":"fixture"}),
            auth: json!({"OPENAI_API_KEY":"fake-placeholder"}),
        },
        ClientConfiguration::ClaudeDesktop {
            profile: json!({"inferenceModels":[{"id":"fixture","name":"Display"}]}),
        },
    ];
    for config in configs {
        let contract = config.contract();
        let state = AppState {
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("route".into()),
                contract: Some(contract),
            }),
            mode_tenant: Some(tenant()),
            ..Default::default()
        };
        assert_eq!(
            rewrite_decision(&state, ProviderMode::Proxy, &config),
            RewriteDecision::SkipRewrite
        );
        assert_eq!(
            rewrite_decision(&state, ProviderMode::Proxy, &configuration()),
            RewriteDecision::RewriteRequired
        );
    }
    let gemini =
        ClientConfiguration::saved(AppId::Gemini, &json!({"env":{},"config":{}}), true).unwrap();
    let custom =
        ClientConfiguration::saved(AppId::Gemini, &json!({"env":{},"config":{}}), false).unwrap();
    assert!(gemini.contract() != custom.contract());
    let grok = ClientConfiguration::saved(AppId::GrokBuild, &json!({"config":""}), true).unwrap();
    assert_eq!(grok.contract().version, 2);
}

#[test]
fn cc04_mode_state_orphan_stack_and_changed_exclusive_fail_closed() {
    let state = AppState {
        stack: StackState { enabled: true, ..Default::default() },
        ..Default::default()
    };
    assert_eq!(observe(&state, &tenant()), Err(ModeStateError::Ownership));
    let mut state =
        AppState { mode: Some(mode()), mode_tenant: Some(tenant()), ..Default::default() };
    state
        .mode
        .as_mut()
        .unwrap()
        .contract
        .as_mut()
        .unwrap()
        .exclusive
        .insert("CLAUDE_CODE_DISABLE_ARTIFACT".into(), json!("changed"));
    assert_eq!(
        rewrite_decision(&state, ProviderMode::Proxy, &configuration()),
        RewriteDecision::RewriteRequired
    );
    let c = ClientConfiguration::Codex {
        config: json!({"model_context_window":128000,"model":"fixture"}),
        auth: json!({}),
    }
    .contract();
    assert_eq!(c.exclusive.get("model_context_window"), Some(&json!(128000)));
    assert!(!c.exclusive.contains_key("model"));
}
